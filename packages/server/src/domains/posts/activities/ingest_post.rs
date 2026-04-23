//! Root Signal ingest orchestrator (TODO §1.1, spec §11 / §12 / §15 / Addendum 01).
//!
//! Entry point is [`ingest_post`] — the handler parses the body once, calls
//! this function, and serialises the returned `IngestResult` as JSON.
//!
//! Lifecycle per request:
//!
//!   1. Validate the envelope against §5 / §6 / §11. Accumulate every field
//!      error into one 422 response — no early returns on the first problem.
//!   2. Reject editor-only fields (§5.11, §11.3).
//!   3. Resolve tags (§10) — service_area/safety hard-fail, unknown topic
//!      auto-creates and flips to `in_review`.
//!   4. Compute `content_hash` (§1.5) and look for an existing match. Hit →
//!      refresh `published_at` and return the existing `post_id`.
//!   5. Resolve source (org or individual dedup). Determine `status` (active
//!      vs in_review) from soft-fail signals.
//!   6. Insert `posts` row. Apply field groups + tags + post_sources (primary
//!      from envelope `source`, extra from Addendum citations).
//!   7. If `revision_of_post_id` is set: archive prior + reflow any active
//!      editions containing it.
//!   8. Build the 201 response shape.
//!
//! The handler wraps this result in idempotency-key storage (see
//! `ApiIdempotencyKey`). This activity is side-effectful but doesn't manage
//! idempotency itself.

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::api::error::{ApiError, ErrorCode, FieldError, FieldErrors};
use crate::common::PostId;
use crate::domains::contacts::Contact as ContactModel;
use crate::domains::posts::activities::{
    content_hash_dedup, individual_dedup, organization_dedup, revision_reflow, tag_resolution,
};
use crate::domains::posts::models::{
    CreatePost, Post, PostDatetimeRecord, PostItem, PostItemInput, PostLinkRecord,
    PostMediaInput, PostMediaRecord, PostMetaRecord, PostPersonRecord, PostScheduleEntry,
    PostScheduleInput, PostSource, PostSourceAttr, PostSourceInsert, PostStatusRecord,
};
use crate::kernel::ServerDeps;

// =============================================================================
// Envelope types — mirror the JSON payload described in ROOT_SIGNAL_DATA_CONTRACT
// =============================================================================

/// The full submission envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct IngestEnvelope {
    pub title: String,
    pub post_type: String,
    pub weight: String,
    pub priority: i32,
    pub body_raw: String,
    #[serde(default)]
    pub body_heavy: Option<String>,
    #[serde(default)]
    pub body_medium: Option<String>,
    #[serde(default)]
    pub body_light: Option<String>,
    #[serde(default)]
    pub body_ast: Option<serde_json::Value>,
    pub published_at: String,
    #[serde(default = "default_language")]
    pub source_language: String,
    #[serde(default)]
    pub is_evergreen: bool,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub zip_code: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,

    pub tags: IngestTags,
    pub source: IngestSource,
    #[serde(default)]
    pub citations: Option<Vec<IngestCitation>>,

    pub meta: IngestMeta,
    #[serde(default)]
    pub field_groups: Option<IngestFieldGroups>,
    #[serde(default)]
    pub editorial: Option<IngestEditorial>,

    // Editor-only fields — hard-rejected if set by Signal (§5.11).
    #[serde(default)]
    pub is_urgent: Option<bool>,
    #[serde(default)]
    pub pencil_mark: Option<String>,
    #[serde(default)]
    pub status: Option<String>,

    #[serde(default)]
    pub submission_type: Option<String>,
}

fn default_language() -> String { "en".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct IngestTags {
    #[serde(default)]
    pub topic: Vec<String>,
    #[serde(default)]
    pub service_area: Vec<String>,
    #[serde(default)]
    pub safety: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestSource {
    pub kind: String,
    #[serde(default)]
    pub source_url: Option<String>,
    pub attribution_line: String,
    #[serde(default)]
    pub extraction_confidence: Option<i32>,
    #[serde(default)]
    pub organization: Option<IngestOrganization>,
    #[serde(default)]
    pub individual: Option<IngestIndividual>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestOrganization {
    pub name: String,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub instagram_handle: Option<String>,
    #[serde(default)]
    pub twitter_handle: Option<String>,
    #[serde(default)]
    pub facebook_handle: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub populations_served: Option<Vec<String>>,
    #[serde(default)]
    pub already_known_org_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestIndividual {
    pub display_name: String,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub platform_url: Option<String>,
    #[serde(default)]
    pub verified_identity: bool,
    #[serde(default)]
    pub consent_to_publish: bool,
    #[serde(default)]
    pub consent_source: Option<String>,
    #[serde(default)]
    pub already_known_individual_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestCitation {
    pub source_url: String,
    pub retrieved_at: String,
    pub content_hash: String,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(default)]
    pub confidence: Option<i32>,
    #[serde(default)]
    pub is_primary: Option<bool>,
    pub kind: String,
    #[serde(default)]
    pub organization: Option<IngestOrganization>,
    #[serde(default)]
    pub individual: Option<IngestIndividual>,
    #[serde(default)]
    pub platform_context: Option<IngestPlatformContext>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestPlatformContext {
    pub platform: String,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub post_type_hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestMeta {
    pub kicker: String,
    pub byline: String,
    #[serde(default)]
    pub deck: Option<String>,
    #[serde(default)]
    pub pull_quote: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IngestFieldGroups {
    #[serde(default)]
    pub datetime: Option<IngestDatetime>,
    #[serde(default)]
    pub schedule: Option<Vec<IngestSchedule>>,
    #[serde(default)]
    pub person: Option<IngestPerson>,
    #[serde(default)]
    pub items: Option<Vec<IngestItem>>,
    #[serde(default)]
    pub contacts: Option<Vec<IngestContact>>,
    #[serde(default)]
    pub link: Option<IngestLink>,
    #[serde(default)]
    pub media: Option<Vec<IngestMedia>>,
    #[serde(default)]
    pub status: Option<IngestStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestDatetime {
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default)]
    pub end_at: Option<String>,
    #[serde(default)]
    pub cost: Option<String>,
    #[serde(default)]
    pub recurring: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestSchedule {
    pub day: String,
    pub opens: String,
    pub closes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestPerson {
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub photo_url: Option<String>,
    #[serde(default)]
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestItem {
    pub name: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestContact {
    pub contact_type: String,
    pub contact_value: String,
    #[serde(default)]
    pub contact_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestLink {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestMedia {
    pub source_image_url: String,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub credit: Option<String>,
    #[serde(default)]
    pub alt_text: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub source_credit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestStatus {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub verified: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestEditorial {
    #[serde(default)]
    pub revision_of_post_id: Option<Uuid>,
    #[serde(default)]
    pub duplicate_of_id: Option<Uuid>,
}

// =============================================================================
// Result shapes (what the handler serialises to JSON)
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct IngestResult {
    pub post_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub individual_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_ids: Option<Vec<Uuid>>,
    pub idempotency_key_seen_before: bool,
    /// True when the submission hit the content-hash path and the server
    /// returned an existing post without inserting. Not part of the public
    /// contract; useful for telemetry.
    #[serde(skip_serializing)]
    pub content_hash_dedup_hit: bool,
}

// =============================================================================
// Validation
// =============================================================================

const VALID_POST_TYPES: &[&str] = &[
    "story", "update", "action", "event", "need", "aid", "person", "business", "reference",
];
const VALID_WEIGHTS: &[&str] = &["heavy", "medium", "light"];

pub fn validate_envelope(env: &IngestEnvelope) -> Result<(), ApiError> {
    let mut errs = FieldErrors::new();

    // -------- editor-only fields --------
    if env.is_urgent.is_some() {
        errs.push(FieldError::new(
            "is_urgent",
            ErrorCode::EditorOnlyField,
            "is_urgent is editor-only; Signal must not set",
        ));
    }
    if env.pencil_mark.is_some() {
        errs.push(FieldError::new(
            "pencil_mark",
            ErrorCode::EditorOnlyField,
            "pencil_mark is editor-only; Signal must not set",
        ));
    }
    if env.status.is_some() {
        errs.push(FieldError::new(
            "status",
            ErrorCode::EditorOnlyField,
            "status is editor-only; Signal must not set",
        ));
    }
    if let Some(t) = &env.submission_type {
        if t != "ingested" {
            errs.push(FieldError::new(
                "submission_type",
                ErrorCode::UnknownValue,
                "submission_type must be 'ingested' on this endpoint",
            ));
        }
    }

    // -------- core identity --------
    let title_chars = env.title.chars().count();
    if title_chars < 20 {
        errs.push(FieldError::new(
            "title",
            ErrorCode::BelowMinLength,
            format!("title is {title_chars} chars; minimum is 20"),
        ));
    } else if title_chars > 120 {
        errs.push(FieldError::new(
            "title",
            ErrorCode::AboveMaxLength,
            format!("title is {title_chars} chars; maximum is 120"),
        ));
    }
    if !VALID_POST_TYPES.contains(&env.post_type.as_str()) {
        errs.push(FieldError::new(
            "post_type",
            ErrorCode::UnknownValue,
            format!(
                "unknown post_type '{}'; expected one of {}",
                env.post_type,
                VALID_POST_TYPES.join("|")
            ),
        ));
    }
    if !VALID_WEIGHTS.contains(&env.weight.as_str()) {
        errs.push(FieldError::new(
            "weight",
            ErrorCode::UnknownValue,
            format!(
                "unknown weight '{}'; expected heavy|medium|light",
                env.weight
            ),
        ));
    }
    if !(0..=100).contains(&env.priority) {
        errs.push(FieldError::new(
            "priority",
            ErrorCode::InvalidFormat,
            format!("priority {} out of 0-100 range", env.priority),
        ));
    }

    // -------- body tiers --------
    let body_raw_chars = env.body_raw.chars().count();
    if body_raw_chars < 250 {
        errs.push(FieldError::new(
            "body_raw",
            ErrorCode::BelowMinLength,
            format!("body_raw is {body_raw_chars} chars; minimum is 250"),
        ));
    }

    match env.weight.as_str() {
        "heavy" => {
            match &env.body_heavy {
                None => errs.push(FieldError::new(
                    "body_heavy",
                    ErrorCode::MissingRequired,
                    "body_heavy is required when weight=heavy",
                )),
                Some(s) if s.chars().count() < 250 => errs.push(FieldError::new(
                    "body_heavy",
                    ErrorCode::BelowMinLength,
                    format!("body_heavy is {} chars; minimum is 250", s.chars().count()),
                )),
                _ => {}
            }
            match &env.body_medium {
                None => errs.push(FieldError::new(
                    "body_medium",
                    ErrorCode::MissingRequired,
                    "body_medium is required when weight∈{heavy,medium}",
                )),
                Some(s) if s.chars().count() < 150 => errs.push(FieldError::new(
                    "body_medium",
                    ErrorCode::BelowMinLength,
                    format!("body_medium is {} chars; minimum is 150", s.chars().count()),
                )),
                _ => {}
            }
            if env.meta.deck.as_ref().map(|d| d.is_empty()).unwrap_or(true) {
                // deck missing on heavy is a SOFT fail — flagged at orchestrator level,
                // not here. Keep this section pure hard-fail.
            }
        }
        "medium" => match &env.body_medium {
            None => errs.push(FieldError::new(
                "body_medium",
                ErrorCode::MissingRequired,
                "body_medium is required when weight∈{heavy,medium}",
            )),
            Some(s) if s.chars().count() < 150 => errs.push(FieldError::new(
                "body_medium",
                ErrorCode::BelowMinLength,
                format!("body_medium is {} chars; minimum is 150", s.chars().count()),
            )),
            _ => {}
        },
        _ => {}
    }
    match &env.body_light {
        None => errs.push(FieldError::new(
            "body_light",
            ErrorCode::MissingRequired,
            "body_light is required on every submission",
        )),
        Some(s) => {
            let c = s.chars().count();
            if c < 40 {
                errs.push(FieldError::new(
                    "body_light",
                    ErrorCode::BelowMinLength,
                    format!("body_light is {c} chars; minimum is 40"),
                ));
            } else if c > 120 {
                errs.push(FieldError::new(
                    "body_light",
                    ErrorCode::AboveMaxLength,
                    format!("body_light is {c} chars; maximum is 120"),
                ));
            }
        }
    }

    // -------- published_at --------
    if DateTime::parse_from_rfc3339(&env.published_at).is_err() {
        errs.push(FieldError::new(
            "published_at",
            ErrorCode::InvalidFormat,
            "published_at must be ISO 8601 with timezone",
        ));
    }

    // -------- source --------
    match env.source.kind.as_str() {
        "editorial" => errs.push(FieldError::new(
            "source.kind",
            ErrorCode::EditorialSourceForbidden,
            "source.kind='editorial' is not ingestible — editorial posts go through the admin UI",
        )),
        "organization" => {
            if env.source.organization.is_none() {
                errs.push(FieldError::new(
                    "source.organization",
                    ErrorCode::OrganizationRequired,
                    "source.kind=organization requires source.organization",
                ));
            }
            if env.source.source_url.is_none() {
                errs.push(FieldError::new(
                    "source.source_url",
                    ErrorCode::SourceUrlRequired,
                    "source.kind=organization requires source.source_url",
                ));
            }
        }
        "individual" => {
            match &env.source.individual {
                None => errs.push(FieldError::new(
                    "source.individual",
                    ErrorCode::MissingRequired,
                    "source.kind=individual requires source.individual",
                )),
                Some(ind) => {
                    if env.source.source_url.is_none() {
                        errs.push(FieldError::new(
                            "source.source_url",
                            ErrorCode::SourceUrlRequired,
                            "source.kind=individual requires source.source_url",
                        ));
                    }
                    if ind.consent_to_publish
                        && ind.platform_url.is_none()
                        && env.source.source_url.is_none()
                    {
                        errs.push(FieldError::new(
                            "source.individual.platform_url",
                            ErrorCode::ConsentWithoutPlatformUrl,
                            "consent_to_publish requires platform_url or source_url",
                        ));
                    }
                }
            }
        }
        other => errs.push(FieldError::new(
            "source.kind",
            ErrorCode::UnknownValue,
            format!("unknown source.kind '{other}'"),
        )),
    }

    // -------- tags --------
    if env.tags.service_area.is_empty() {
        errs.push(FieldError::new(
            "tags.service_area",
            ErrorCode::MissingRequired,
            "at least one service_area tag required",
        ));
    }
    if env.tags.topic.is_empty() {
        errs.push(FieldError::new(
            "tags.topic",
            ErrorCode::MissingRequired,
            "at least one topic tag required",
        ));
    }

    // -------- meta --------
    if env.meta.kicker.trim().is_empty() {
        errs.push(FieldError::new(
            "meta.kicker",
            ErrorCode::MissingRequired,
            "meta.kicker is required",
        ));
    }
    if env.meta.byline.trim().is_empty() {
        errs.push(FieldError::new(
            "meta.byline",
            ErrorCode::MissingRequired,
            "meta.byline is required",
        ));
    }

    // -------- per-post-type field groups (§6) --------
    validate_field_groups_for_type(env, &mut errs);

    // -------- coordinates --------
    if let (Some(lat), Some(lng)) = (env.latitude, env.longitude) {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
            errs.push(FieldError::new(
                "latitude/longitude",
                ErrorCode::InvalidCoordinates,
                "coordinates out of range",
            ));
        } else if lat == 0.0 && lng == 0.0 {
            errs.push(FieldError::new(
                "latitude/longitude",
                ErrorCode::InvalidCoordinates,
                "(0,0) rejected as placeholder",
            ));
        }
    }

    // -------- citations (Addendum 01) --------
    if let Some(citations) = &env.citations {
        validate_citations(env, citations, &mut errs);
    }

    errs.into_result()
}

fn validate_field_groups_for_type(env: &IngestEnvelope, errs: &mut FieldErrors) {
    let groups = env.field_groups.as_ref();
    let missing = |name: &str, errs: &mut FieldErrors| {
        errs.push(FieldError::new(
            format!("field_groups.{name}"),
            ErrorCode::PostTypeGroupMissing,
            format!("{} requires field_groups.{name}", env.post_type),
        ));
    };

    match env.post_type.as_str() {
        "event" => {
            if groups.and_then(|g| g.datetime.as_ref()).is_none() {
                missing("datetime", errs);
            }
            if env.location.is_none()
                && env.zip_code.is_none()
                && groups
                    .and_then(|g| g.contacts.as_ref())
                    .map(|c| !c.iter().any(|x| x.contact_type == "address"))
                    .unwrap_or(true)
            {
                errs.push(FieldError::new(
                    "location",
                    ErrorCode::PostTypeGroupMissing,
                    "event requires location / zip_code / address contact",
                ));
            }
            if groups
                .map(|g| g.contacts.as_ref().map(|c| c.is_empty()).unwrap_or(true)
                    && g.link.is_none())
                .unwrap_or(true)
            {
                errs.push(FieldError::new(
                    "field_groups.contacts|link",
                    ErrorCode::PostTypeGroupMissing,
                    "event requires contacts or a link for RSVP",
                ));
            }
        }
        "action" => {
            if groups
                .and_then(|g| g.link.as_ref())
                .and_then(|l| l.url.as_ref())
                .is_none()
            {
                missing("link.url", errs);
            }
        }
        "need" | "aid" => {
            if groups
                .and_then(|g| g.items.as_ref())
                .map(|i| i.is_empty())
                .unwrap_or(true)
            {
                missing("items", errs);
            }
            if groups
                .and_then(|g| g.contacts.as_ref())
                .map(|c| c.is_empty())
                .unwrap_or(true)
            {
                missing("contacts", errs);
            }
            if groups.and_then(|g| g.status.as_ref()).is_none() {
                missing("status", errs);
            }
        }
        "person" => {
            if groups.and_then(|g| g.person.as_ref()).is_none() {
                missing("person", errs);
            }
        }
        "business" => {
            if groups
                .and_then(|g| g.contacts.as_ref())
                .map(|c| c.is_empty())
                .unwrap_or(true)
            {
                missing("contacts", errs);
            }
            if groups
                .and_then(|g| g.schedule.as_ref())
                .map(|s| s.is_empty())
                .unwrap_or(true)
            {
                missing("schedule", errs);
            }
            if env.location.is_none() && env.zip_code.is_none() {
                errs.push(FieldError::new(
                    "location",
                    ErrorCode::PostTypeGroupMissing,
                    "business requires location or zip_code",
                ));
            }
        }
        "reference" => {
            if groups
                .and_then(|g| g.items.as_ref())
                .map(|i| i.is_empty())
                .unwrap_or(true)
            {
                missing("items", errs);
            }
        }
        "update" => {
            let has_contacts = groups
                .and_then(|g| g.contacts.as_ref())
                .map(|c| !c.is_empty())
                .unwrap_or(false);
            let has_link = groups
                .and_then(|g| g.link.as_ref())
                .and_then(|l| l.url.as_ref())
                .is_some();
            if !has_contacts && !has_link {
                errs.push(FieldError::new(
                    "field_groups.contacts|link",
                    ErrorCode::PostTypeGroupMissing,
                    "update requires contacts or a link",
                ));
            }
        }
        _ => {}
    }
}

fn validate_citations(env: &IngestEnvelope, citations: &[IngestCitation], errs: &mut FieldErrors) {
    if citations.len() > 10 {
        errs.push(FieldError::new(
            "citations",
            ErrorCode::TooManyCitations,
            format!("{} citations submitted; max is 10", citations.len()),
        ));
    }

    for (i, c) in citations.iter().enumerate() {
        let prefix = format!("citations[{i}]");
        if !c.content_hash.starts_with("sha256:") || c.content_hash.len() != 71 {
            errs.push(FieldError::new(
                format!("{prefix}.content_hash"),
                ErrorCode::CitationHashFormat,
                "content_hash must be 'sha256:' followed by 64 hex chars",
            ));
        }
        if let Err(_) = DateTime::parse_from_rfc3339(&c.retrieved_at) {
            errs.push(FieldError::new(
                format!("{prefix}.retrieved_at"),
                ErrorCode::InvalidRetrievedAt,
                "retrieved_at must be ISO 8601",
            ));
        } else if let Ok(t) = DateTime::parse_from_rfc3339(&c.retrieved_at) {
            if t.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5) {
                errs.push(FieldError::new(
                    format!("{prefix}.retrieved_at"),
                    ErrorCode::InvalidRetrievedAt,
                    "retrieved_at is in the future",
                ));
            }
        }
        match c.kind.as_str() {
            "organization" => {
                if c.organization.is_none() {
                    errs.push(FieldError::new(
                        format!("{prefix}.organization"),
                        ErrorCode::CitationMissingRequired,
                        "citation.kind=organization requires organization block",
                    ));
                }
            }
            "individual" => {
                if c.individual.is_none() {
                    errs.push(FieldError::new(
                        format!("{prefix}.individual"),
                        ErrorCode::CitationMissingRequired,
                        "citation.kind=individual requires individual block",
                    ));
                }
            }
            "editorial" => errs.push(FieldError::new(
                format!("{prefix}.kind"),
                ErrorCode::CitationEditorialForbidden,
                "citation.kind='editorial' is not valid",
            )),
            other => errs.push(FieldError::new(
                format!("{prefix}.kind"),
                ErrorCode::UnknownValue,
                format!("unknown citation.kind '{other}'"),
            )),
        }
    }

    // Primary-match check — spec §2.1 of addendum.
    let primary = citations.iter().find(|c| c.is_primary == Some(true));
    let chosen = primary.or(citations.first());
    if let (Some(primary), Some(submission_url)) = (chosen, env.source.source_url.as_ref()) {
        if primary.source_url != *submission_url {
            errs.push(FieldError::new(
                "source.source_url",
                ErrorCode::CitationPrimaryMismatch,
                format!(
                    "source.source_url does not match primary citation source_url"
                ),
            ));
        }
    }
}

// =============================================================================
// Orchestrator
// =============================================================================

pub async fn ingest_post(
    env: IngestEnvelope,
    deps: &ServerDeps,
) -> Result<IngestResult, ApiError> {
    validate_envelope(&env)?;
    let pool = &deps.db_pool;

    // ---- tags ----
    let tag_res = tag_resolution::resolve_tags(
        &env.tags.topic,
        &env.tags.service_area,
        &env.tags.safety,
        pool,
    )
    .await?;
    if !tag_res.errors.is_empty() {
        return Err(ApiError::Validation(tag_res.errors));
    }

    // ---- parse published_at (validated earlier) ----
    let published_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&env.published_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| {
            ApiError::Validation(vec![FieldError::new(
                "published_at",
                ErrorCode::InvalidFormat,
                "published_at failed to parse",
            )])
        })?;

    // ---- content-hash dedup ----
    let content_hash = content_hash_dedup::compute_content_hash(
        &env.title,
        env.source.source_url.as_deref(),
        Some(published_at),
        &tag_res.service_area_slugs,
    );
    if let Some(existing_id) = content_hash_dedup::find_existing_by_hash(&content_hash, pool).await?
    {
        content_hash_dedup::refresh_existing(existing_id, pool).await?;
        let existing = Post::find_by_id(PostId::from_uuid(existing_id), pool).await?;
        return Ok(IngestResult {
            post_id: existing_id,
            status: existing
                .map(|p| p.status)
                .unwrap_or_else(|| "active".to_string()),
            organization_id: None,
            individual_id: None,
            citation_ids: None,
            idempotency_key_seen_before: false,
            content_hash_dedup_hit: true,
        });
    }

    // ---- primary source dedup ----
    let mut soft_flags = SoftFlags::default();
    if env.source.extraction_confidence.map(|c| c < 60).unwrap_or(false) {
        soft_flags.low_confidence = true;
    }
    let duplicate_of = env.editorial.as_ref().and_then(|e| e.duplicate_of_id);
    if duplicate_of.is_some() {
        soft_flags.possible_duplicate = true;
    }
    if env.weight == "heavy" && env.meta.deck.as_ref().map(|d| d.trim().is_empty()).unwrap_or(true) {
        soft_flags.deck_missing_on_heavy = true;
    }
    if tag_res.unknown_topic_auto_created {
        soft_flags.unknown_topic = true;
    }

    let (organization_id, individual_id, primary_source) = resolve_primary_source(
        &env, &mut soft_flags, pool,
    )
    .await?;

    // ---- insert posts row ----
    let status = if soft_flags.any() { "in_review" } else { "active" };
    let revision_of = env.editorial.as_ref().and_then(|e| e.revision_of_post_id);

    let post = Post::create(
        CreatePost::builder()
            .title(env.title.clone())
            .body_raw(env.body_raw.clone())
            .post_type(env.post_type.clone())
            .weight(env.weight.clone())
            .priority(env.priority)
            .is_urgent(false)
            .location(env.location.clone())
            .status(status.to_string())
            .source_language(env.source_language.clone())
            .submission_type(Some("ingested".to_string()))
            .submitted_by_id(None::<Uuid>)
            .revision_of_post_id(revision_of.map(PostId::from_uuid))
            .translation_of_id(None::<PostId>)
            .published_at(Some(published_at))
            .build(),
        pool,
    )
    .await?;
    let post_id = post.id;
    let post_uuid = post_id.into_uuid();

    // Body tiers and body_ast aren't on CreatePost — write them in a follow-up.
    apply_body_tiers(
        post_uuid,
        env.body_heavy.as_deref(),
        env.body_medium.as_deref(),
        env.body_light.as_deref(),
        env.body_ast.as_ref(),
        env.zip_code.as_deref(),
        env.latitude,
        env.longitude,
        env.is_evergreen,
        duplicate_of,
        pool,
    )
    .await?;

    // ---- content hash ----
    content_hash_dedup::set_content_hash(post_uuid, &content_hash, pool).await?;

    // ---- tags ----
    tag_resolution::apply_tags(post_id, &tag_res, pool).await?;

    // ---- meta / field groups ----
    apply_field_groups(post_uuid, &env, pool).await?;

    // ---- post_sources: primary + additional citations ----
    let mut citation_ids: Vec<Uuid> = Vec::new();
    let primary_row = PostSource::insert_full(
        PostSourceInsert {
            post_id,
            source_type: &primary_source.source_type,
            source_id: primary_source.source_id,
            source_url: env.source.source_url.as_deref(),
            content_hash: None,
            snippet: None,
            confidence: env.source.extraction_confidence,
            platform_id: None,
            platform_post_type_hint: None,
            is_primary: true,
            retrieved_at: None,
        },
        pool,
    )
    .await?;
    citation_ids.push(primary_row.id.into_uuid());

    if let Some(citations) = env.citations.clone() {
        for citation in citations.iter().filter(|c| c.is_primary != Some(true)) {
            // Same citation as primary (by source_url) — skip; the primary row covers it.
            if Some(&citation.source_url) == env.source.source_url.as_ref() {
                continue;
            }
            let cit_row = persist_citation(post_id, citation, pool).await?;
            citation_ids.push(cit_row);
        }
    }

    // ---- public attribution line ----
    let source_name = env
        .source
        .organization
        .as_ref()
        .map(|o| o.name.clone())
        .or_else(|| env.source.individual.as_ref().map(|i| i.display_name.clone()));
    PostSourceAttr::upsert(
        post_uuid,
        source_name.as_deref(),
        Some(env.source.attribution_line.as_str()),
        pool,
    )
    .await?;

    // ---- revision reflow ----
    if let Some(prior) = revision_of {
        let _ = revision_reflow::archive_and_reflow(prior, deps).await
            .map_err(|e| {
                tracing::warn!(error = %e, prior_post_id = %prior, "revision reflow failed");
                e
            });
    }

    info!(
        post_id = %post_uuid,
        status = %status,
        post_type = %env.post_type,
        "root-signal ingest completed"
    );

    Ok(IngestResult {
        post_id: post_uuid,
        status: status.to_string(),
        organization_id,
        individual_id,
        citation_ids: if env.citations.is_some() {
            Some(citation_ids)
        } else {
            None
        },
        idempotency_key_seen_before: false,
        content_hash_dedup_hit: false,
    })
}

#[derive(Debug, Default)]
struct SoftFlags {
    low_confidence: bool,
    possible_duplicate: bool,
    deck_missing_on_heavy: bool,
    unknown_topic: bool,
    source_stale: bool,
    individual_no_consent: bool,
}

impl SoftFlags {
    fn any(&self) -> bool {
        self.low_confidence
            || self.possible_duplicate
            || self.deck_missing_on_heavy
            || self.unknown_topic
            || self.source_stale
            || self.individual_no_consent
    }
}

struct PrimarySource {
    source_type: String,
    source_id: Uuid,
}

async fn resolve_primary_source(
    env: &IngestEnvelope,
    flags: &mut SoftFlags,
    pool: &sqlx::PgPool,
) -> Result<(Option<Uuid>, Option<Uuid>, PrimarySource), ApiError> {
    match env.source.kind.as_str() {
        "organization" => {
            let org_block = env
                .source
                .organization
                .as_ref()
                .expect("organization presence validated upstream");
            let resolved = organization_dedup::resolve_organization(
                organization_dedup::OrganizationSubmission {
                    name: &org_block.name,
                    website: org_block.website.as_deref(),
                    description: None,
                    already_known_org_id: org_block.already_known_org_id,
                },
                pool,
            )
            .await?;
            if resolved.stale {
                flags.source_stale = true;
            }
            Ok((
                Some(resolved.org.id.into_uuid()),
                None,
                PrimarySource {
                    source_type: resolved.source_type,
                    source_id: resolved.source_id,
                },
            ))
        }
        "individual" => {
            let ind_block = env
                .source
                .individual
                .as_ref()
                .expect("individual presence validated upstream");
            if !ind_block.consent_to_publish {
                flags.individual_no_consent = true;
            }
            let resolved = individual_dedup::resolve_individual(
                individual_dedup::IndividualSubmission {
                    display_name: &ind_block.display_name,
                    handle: ind_block.handle.as_deref(),
                    platform: ind_block.platform.as_deref(),
                    platform_url: ind_block.platform_url.as_deref(),
                    verified_identity: ind_block.verified_identity,
                    consent_to_publish: ind_block.consent_to_publish,
                    consent_source: ind_block.consent_source.as_deref(),
                    already_known_individual_id: ind_block.already_known_individual_id,
                },
                pool,
            )
            .await?;
            Ok((
                None,
                Some(resolved.individual.id.into_uuid()),
                PrimarySource {
                    source_type: resolved.source_type,
                    source_id: resolved.source_id,
                },
            ))
        }
        _ => unreachable!("source.kind validated upstream"),
    }
}

async fn persist_citation(
    post_id: PostId,
    citation: &IngestCitation,
    pool: &sqlx::PgPool,
) -> Result<Uuid, ApiError> {
    let (source_type, source_id) = match citation.kind.as_str() {
        "organization" => {
            let org = citation.organization.as_ref().expect("validated");
            let resolved = organization_dedup::resolve_organization(
                organization_dedup::OrganizationSubmission {
                    name: &org.name,
                    website: org.website.as_deref(),
                    description: None,
                    already_known_org_id: org.already_known_org_id,
                },
                pool,
            )
            .await?;
            (resolved.source_type, resolved.source_id)
        }
        "individual" => {
            let ind = citation.individual.as_ref().expect("validated");
            let resolved = individual_dedup::resolve_individual(
                individual_dedup::IndividualSubmission {
                    display_name: &ind.display_name,
                    handle: ind.handle.as_deref(),
                    platform: ind.platform.as_deref(),
                    platform_url: ind.platform_url.as_deref(),
                    verified_identity: ind.verified_identity,
                    consent_to_publish: ind.consent_to_publish,
                    consent_source: ind.consent_source.as_deref(),
                    already_known_individual_id: ind.already_known_individual_id,
                },
                pool,
            )
            .await?;
            (resolved.source_type, resolved.source_id)
        }
        _ => unreachable!("citation.kind validated upstream"),
    };

    let retrieved_at = DateTime::parse_from_rfc3339(&citation.retrieved_at)
        .ok()
        .map(|d| d.with_timezone(&Utc));
    let platform_id = citation
        .platform_context
        .as_ref()
        .and_then(|p| p.platform_id.as_deref());
    let platform_post_type_hint = citation
        .platform_context
        .as_ref()
        .and_then(|p| p.post_type_hint.as_deref());

    let row = PostSource::insert_full(
        PostSourceInsert {
            post_id,
            source_type: &source_type,
            source_id,
            source_url: Some(&citation.source_url),
            content_hash: Some(&citation.content_hash),
            snippet: citation.snippet.as_deref(),
            confidence: citation.confidence,
            platform_id,
            platform_post_type_hint,
            is_primary: false,
            retrieved_at,
        },
        pool,
    )
    .await?;
    Ok(row.id.into_uuid())
}

#[allow(clippy::too_many_arguments)]
async fn apply_body_tiers(
    post_id: Uuid,
    heavy: Option<&str>,
    medium: Option<&str>,
    light: Option<&str>,
    body_ast: Option<&serde_json::Value>,
    zip_code: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    is_evergreen: bool,
    duplicate_of: Option<Uuid>,
    pool: &sqlx::PgPool,
) -> Result<(), ApiError> {
    let lat_dec = latitude.and_then(|f| Decimal::try_from(f).ok());
    let lng_dec = longitude.and_then(|f| Decimal::try_from(f).ok());
    sqlx::query(
        r#"
        UPDATE posts
        SET
            body_heavy  = $2,
            body_medium = $3,
            body_light  = $4,
            body_ast    = $5,
            zip_code    = $6,
            latitude    = $7,
            longitude   = $8,
            is_evergreen = $9,
            duplicate_of_id = $10,
            updated_at  = NOW()
        WHERE id = $1
        "#,
    )
    .bind(post_id)
    .bind(heavy)
    .bind(medium)
    .bind(light)
    .bind(body_ast)
    .bind(zip_code)
    .bind(lat_dec)
    .bind(lng_dec)
    .bind(is_evergreen)
    .bind(duplicate_of)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}

async fn apply_field_groups(
    post_id: Uuid,
    env: &IngestEnvelope,
    pool: &sqlx::PgPool,
) -> Result<(), ApiError> {
    // Meta (1:1)
    PostMetaRecord::upsert(
        post_id,
        Some(env.meta.kicker.as_str()),
        Some(env.meta.byline.as_str()),
        env.meta.deck.as_deref(),
        env.meta.updated.as_deref(),
        pool,
    )
    .await?;

    // Pull quote currently lives on post_meta via migration 216; meta upsert
    // overwrites it. Handled via a follow-up statement because the shared
    // `upsert` doesn't take pull_quote yet.
    if let Some(pq) = env.meta.pull_quote.as_deref() {
        sqlx::query("UPDATE post_meta SET pull_quote = $2 WHERE post_id = $1")
            .bind(post_id)
            .bind(pq)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    }

    let Some(groups) = &env.field_groups else {
        return Ok(());
    };

    if let Some(dt) = &groups.datetime {
        let start = dt
            .start_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        let end = dt
            .end_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        PostDatetimeRecord::upsert(
            post_id,
            start,
            end,
            dt.cost.as_deref(),
            dt.recurring,
            pool,
        )
        .await?;
    }

    if let Some(sched) = &groups.schedule {
        if !sched.is_empty() {
            let inputs: Vec<PostScheduleInput> = sched
                .iter()
                .map(|e| PostScheduleInput {
                    day: e.day.clone(),
                    opens: e.opens.clone(),
                    closes: e.closes.clone(),
                })
                .collect();
            PostScheduleEntry::replace_all(post_id, &inputs, pool).await?;
        }
    }

    if let Some(p) = &groups.person {
        PostPersonRecord::upsert(
            post_id,
            Some(p.name.as_str()),
            p.role.as_deref(),
            p.bio.as_deref(),
            p.photo_url.as_deref(),
            p.quote.as_deref(),
            None,
            pool,
        )
        .await?;
    }

    if let Some(items) = &groups.items {
        if !items.is_empty() {
            let inputs: Vec<PostItemInput> = items
                .iter()
                .map(|i| PostItemInput {
                    name: i.name.clone(),
                    detail: i.detail.clone(),
                })
                .collect();
            PostItem::replace_all(post_id, &inputs, pool).await?;
        }
    }

    if let Some(contacts) = &groups.contacts {
        if !contacts.is_empty() {
            // Wipe + re-insert; ingest is the source of truth for this post's
            // contacts block.
            ContactModel::delete_all_for_post(PostId::from_uuid(post_id), pool).await?;
            for (i, c) in contacts.iter().enumerate() {
                let ctype = c
                    .contact_type
                    .parse::<crate::domains::contacts::models::contact::ContactType>()
                    .map_err(|_| {
                        ApiError::Validation(vec![FieldError::new(
                            format!("field_groups.contacts[{i}].contact_type"),
                            ErrorCode::UnknownValue,
                            format!("unknown contact_type '{}'", c.contact_type),
                        )])
                    })?;
                ContactModel::create(
                    "post",
                    post_id,
                    ctype,
                    c.contact_value.clone(),
                    c.contact_label.clone(),
                    Some(i as i32),
                    pool,
                )
                .await?;
            }
        }
    }

    if let Some(link) = &groups.link {
        let deadline = link.deadline.as_deref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.date_naive())
                .or_else(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        });
        PostLinkRecord::upsert(
            post_id,
            link.label.as_deref(),
            link.url.as_deref(),
            deadline,
            pool,
        )
        .await?;
    }

    if let Some(media) = &groups.media {
        if !media.is_empty() {
            let inputs: Vec<PostMediaInput> = media
                .iter()
                .map(|m| PostMediaInput {
                    image_url: Some(m.source_image_url.clone()),
                    caption: m.caption.clone(),
                    credit: m.credit.clone().or(m.source_credit.clone()),
                    alt_text: m.alt_text.clone(),
                    media_id: None,
                })
                .collect();
            PostMediaRecord::replace_all(post_id, &inputs, pool).await?;
        }
    }

    if let Some(status) = &groups.status {
        let verified = status
            .verified
            .as_ref()
            .map(|v| {
                if v.is_boolean() {
                    v.as_bool().unwrap_or(false).to_string()
                } else {
                    v.as_str().unwrap_or("").to_string()
                }
            });
        PostStatusRecord::upsert(post_id, status.state.as_deref(), verified.as_deref(), pool)
            .await?;
    }

    Ok(())
}
