//! Individual-source dedup ladder (spec §7.2).
//!
//!   1. `already_known_individual_id` hint → direct lookup.
//!   2. `(platform, handle)` exact match.
//!   3. `platform_url` match.
//!   4. Insert.
//!
//! On match, NULL columns on the stored row are filled in from the
//! submission. `consent_to_publish` is monotonic (once true, stays true).
//!
//! `consent_to_publish = false` is a soft-fail the orchestrator surfaces —
//! this activity just returns the row.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::SourceIndividualId;
use crate::domains::posts::models::{SourceIndividual, SourceIndividualInput};

#[derive(Debug, Clone)]
pub struct IndividualSubmission<'a> {
    pub display_name: &'a str,
    pub handle: Option<&'a str>,
    pub platform: Option<&'a str>,
    pub platform_url: Option<&'a str>,
    pub verified_identity: bool,
    pub consent_to_publish: bool,
    pub consent_source: Option<&'a str>,
    pub already_known_individual_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ResolvedIndividual {
    pub individual: SourceIndividual,
    /// The `post_sources.source_id` points at `source_individuals.id` directly
    /// (no parent `sources` row for individuals — see the DATA_MODEL doc).
    pub source_id: Uuid,
    pub source_type: String,
    pub created: bool,
}

pub async fn resolve_individual(
    submission: IndividualSubmission<'_>,
    pool: &PgPool,
) -> Result<ResolvedIndividual> {
    let input = to_input(&submission);

    // ------ Step 1 — client-hinted id -----------------------------------------
    if let Some(hint) = submission.already_known_individual_id {
        if let Some(existing) =
            SourceIndividual::find_by_id(SourceIndividualId::from_uuid(hint), pool).await?
        {
            let enriched = SourceIndividual::enrich(existing.id, &input, pool).await?;
            return Ok(finalise(enriched, false));
        }
        tracing::debug!(?hint, "unknown already_known_individual_id; falling back to dedup ladder");
    }

    // ------ Step 2 — (platform, handle) ---------------------------------------
    if let (Some(platform), Some(handle)) = (submission.platform, submission.handle) {
        if let Some(existing) =
            SourceIndividual::find_by_platform_handle(platform, handle, pool).await?
        {
            let enriched = SourceIndividual::enrich(existing.id, &input, pool).await?;
            return Ok(finalise(enriched, false));
        }
    }

    // ------ Step 3 — platform_url ---------------------------------------------
    if let Some(platform_url) = submission.platform_url {
        if let Some(existing) = SourceIndividual::find_by_platform_url(platform_url, pool).await? {
            let enriched = SourceIndividual::enrich(existing.id, &input, pool).await?;
            return Ok(finalise(enriched, false));
        }
    }

    // ------ Step 4 — insert ---------------------------------------------------
    let inserted = SourceIndividual::insert(input, pool).await?;
    Ok(finalise(inserted, true))
}

fn finalise(individual: SourceIndividual, created: bool) -> ResolvedIndividual {
    let source_type = individual
        .platform
        .clone()
        .unwrap_or_else(|| "individual".to_string());
    let source_id = individual.id.into_uuid();
    ResolvedIndividual {
        individual,
        source_id,
        source_type,
        created,
    }
}

fn to_input(s: &IndividualSubmission<'_>) -> SourceIndividualInput {
    SourceIndividualInput {
        display_name: s.display_name.to_string(),
        handle: s.handle.map(|h| h.to_string()),
        platform: s.platform.map(|p| p.to_string()),
        platform_url: s.platform_url.map(|u| u.to_string()),
        verified_identity: s.verified_identity,
        consent_to_publish: s.consent_to_publish,
        consent_source: s.consent_source.map(|c| c.to_string()),
        consent_captured_at: if s.consent_to_publish {
            Some(chrono::Utc::now())
        } else {
            None
        },
    }
}
