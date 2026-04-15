//! Posts route file
//!
//! Merges the Posts stateless service and PostObject virtual object handlers
//! into a single Axum route file.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::auth::{AdminUser, OptionalUser};
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::common::{PaginationArgs, PostId, ScheduleId};
use crate::domains::contacts::Contact;
use crate::domains::editions::Edition;
use crate::domains::locations::models::ZipCode;
use crate::domains::notes::models::note::Note;
use crate::domains::posts::activities;
use crate::domains::posts::activities::schedule::ScheduleParams;
use crate::domains::posts::activities::tags::TagInput;
use crate::domains::posts::data::types::SubmitPostInput;
use crate::domains::posts::models::post::PostFilters;
use crate::domains::posts::models::post_report::{PostReportRecord, PostReportWithDetails};
use crate::domains::posts::models::Post;
use crate::domains::posts::models::{
    PostMediaRecord, PostMetaRecord, PostPersonRecord, PostLinkRecord,
    PostSourceAttr, PostDatetimeRecord, PostStatusRecord,
};
use crate::domains::schedules::models::Schedule;
use crate::domains::tag::models::tag::Tag;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types — Posts service (stateless)
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Deserialize)]
pub struct ListPostsRequest {
    pub status: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
    pub search: Option<String>,
    pub zip_code: Option<String>,
    pub radius_miles: Option<f64>,
    pub post_type: Option<String>,
    pub submission_type: Option<String>,
    pub exclude_submission_type: Option<String>,
    pub county_id: Option<Uuid>,
    pub statewide_only: Option<bool>,
    pub first: Option<i32>,
    pub offset: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListPostsForEditionRequest {
    pub edition_id: Uuid,
    pub slotted_filter: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NearbySearchRequest {
    pub zip_code: String,
    pub radius_miles: Option<f64>,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitPostRequest {
    pub title: String,
    pub body_raw: String,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub contact_website: Option<String>,
    pub is_urgent: Option<bool>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpcomingEventsRequest {
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PendingRevisionsRequest {
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListReportsRequest {
    pub status: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicListRequest {
    pub post_type: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub zip_code: Option<String>,
    pub radius_miles: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListPostsByOrganizationRequest {
    pub organization_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicFiltersRequest {}

#[derive(Debug, Clone, Deserialize)]
pub struct PostStatsRequest {
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackfillLocationsRequest {
    pub batch_size: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulesForEntityRequest {
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpireStalePostsRequest {}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub body_raw: String,
    pub post_type: Option<String>,
    pub weight: Option<String>,
    pub priority: Option<i32>,
    pub is_urgent: Option<bool>,
    pub location: Option<String>,
}

// =============================================================================
// Request types — Post virtual object (keyed by post_id)
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct GetPostRequest {
    #[serde(default)]
    pub show_private: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApprovePostRequest {}

#[derive(Debug, Clone, Deserialize)]
pub struct EditApproveRequest {
    pub title: Option<String>,
    pub body_raw: Option<String>,
    pub is_urgent: Option<bool>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RejectPostRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportPostRequest {
    pub reason: String,
    pub category: String,
    pub reporter_email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResolveReportRequest {
    pub report_id: Uuid,
    pub resolution_notes: Option<String>,
    pub action_taken: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DismissReportRequest {
    pub report_id: Uuid,
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTagsRequest {
    pub tags: Vec<TagInputData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagInputData {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddTagRequest {
    pub tag_kind: String,
    pub tag_value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoveTagRequest {
    pub tag_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddContactRequest {
    pub contact_type: String,
    pub contact_value: String,
    pub contact_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoveContactRequest {
    pub contact_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddScheduleRequest {
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub day_of_week: Option<i32>,
    pub timezone: Option<String>,
    pub is_all_day: Option<bool>,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateScheduleRequest {
    pub schedule_id: Uuid,
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub day_of_week: Option<i32>,
    pub timezone: Option<String>,
    pub is_all_day: Option<bool>,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteScheduleRequest {
    pub schedule_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePostContentRequest {
    pub title: Option<String>,
    pub body_raw: Option<String>,
    pub body_ast: Option<serde_json::Value>,
    pub post_type: Option<String>,
    pub weight: Option<String>,
    pub priority: Option<i32>,
    pub is_urgent: Option<bool>,
    pub location: Option<String>,
    pub zip_code: Option<String>,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTagResult {
    pub id: Uuid,
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostContactResult {
    pub id: Uuid,
    pub contact_type: String,
    pub contact_value: String,
    pub contact_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmittedByInfo {
    pub submitter_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostScheduleResult {
    pub id: Uuid,
    pub day_of_week: Option<i32>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub timezone: String,
    pub notes: Option<String>,
    pub rrule: Option<String>,
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub is_all_day: bool,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrgentNoteInfo {
    pub content: String,
    pub cta_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResult {
    pub id: Uuid,
    pub title: String,
    pub body_raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_ast: Option<serde_json::Value>,
    pub status: String,
    pub post_type: String,
    pub is_urgent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pencil_mark: Option<String>,
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_code: Option<String>,
    pub submission_type: Option<String>,
    pub weight: String,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_heavy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_light: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<PostTagResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_by: Option<SubmittedByInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedules: Option<Vec<PostScheduleResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<PostContactResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_urgent_notes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgent_notes: Option<Vec<UrgentNoteInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_miles: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_of_post_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_of_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of_id: Option<Uuid>,
    pub source_language: String,
}

impl From<Post> for PostResult {
    fn from(p: Post) -> Self {
        Self {
            id: p.id.into_uuid(),
            title: p.title,
            body_raw: p.body_raw,
            body_ast: p.body_ast,
            status: p.status,
            post_type: p.post_type,
            is_urgent: p.is_urgent,
            pencil_mark: p.pencil_mark,
            location: p.location,
            zip_code: p.zip_code,
            submission_type: p.submission_type,
            weight: p.weight,
            priority: p.priority,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
            published_at: p.published_at.map(|dt| dt.to_rfc3339()),
            body_heavy: p.body_heavy,
            body_medium: p.body_medium,
            body_light: p.body_light,
            tags: None,
            submitted_by: None,
            schedules: Some(vec![]),
            contacts: Some(vec![]),
            organization_id: None,
            organization_name: None,
            has_urgent_notes: None,
            urgent_notes: None,
            distance_miles: None,
            latitude: p.latitude.and_then(|d| d.to_f64()),
            longitude: p.longitude.and_then(|d| d.to_f64()),
            revision_of_post_id: p.revision_of_post_id.map(|id| id.into_uuid()),
            translation_of_id: p.translation_of_id.map(|id| id.into_uuid()),
            duplicate_of_id: p.duplicate_of_id.map(|id| id.into_uuid()),
            source_language: p.source_language,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostListResult {
    pub posts: Vec<PostResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyPostResult {
    pub post: PostResult,
    pub distance_miles: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchResults {
    pub results: Vec<NearbyPostResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRevisionsResult {
    pub posts: Vec<PostResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPostResult {
    pub post_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDetailResult {
    pub id: Uuid,
    pub post_id: Uuid,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub reporter_email: Option<String>,
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportListResult {
    pub reports: Vec<ReportDetailResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPostResult {
    pub id: Uuid,
    pub title: String,
    pub body_raw: String,
    pub status: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingEventsResult {
    pub events: Vec<EventPostResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDetailResult {
    pub id: Uuid,
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
    pub day_of_week: Option<i32>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub timezone: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub notes: Option<String>,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleListResult {
    pub schedules: Vec<ScheduleDetailResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    pub id: Uuid,
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPostResult {
    pub id: Uuid,
    pub title: String,
    pub body_raw: String,
    pub body_light: Option<String>,
    pub location: Option<String>,
    pub post_type: String,
    pub is_urgent: bool,
    pub created_at: String,
    pub published_at: Option<String>,
    pub tags: Vec<PublicTagResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub urgent_notes: Vec<UrgentNoteInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_miles: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicTagResult {
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicListResult {
    pub posts: Vec<PublicPostResult>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOption {
    pub value: String,
    pub display_name: String,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTypeOption {
    pub value: String,
    pub display_name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicFiltersResult {
    pub post_types: Vec<PostTypeOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostStatsResult {
    pub total: i64,
    pub stories: i64,
    pub notices: i64,
    pub exchanges: i64,
    pub events: i64,
    pub spotlights: i64,
    pub references: i64,
    pub user_submitted: i64,
    pub ingested: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillLocationsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpireStalePostsResult {
    pub expired_count: u64,
}

// Post object report result (different shape from service-level ReportDetailResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResult {
    pub id: Uuid,
    pub post_id: Uuid,
    pub reason: String,
    pub category: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostReportListResult {
    pub reports: Vec<ReportResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalPostResult {
    pub post: Option<PostResult>,
}

// =============================================================================
// Helper: batch-load public tags and urgent notes for a set of post IDs
// =============================================================================

pub(crate) async fn load_tags_and_notes(
    post_ids: &[Uuid],
    deps: &ServerDeps,
) -> ApiResult<(
    HashMap<Uuid, Vec<PublicTagResult>>,
    HashMap<Uuid, Vec<UrgentNoteInfo>>,
)> {
    let tag_rows = Tag::find_public_for_post_ids(post_ids, &deps.db_pool).await?;

    let urgent_rows = Note::find_urgent_note_content_for_posts(post_ids, &deps.db_pool)
        .await
        .unwrap_or_default();

    let mut tags_by_post: HashMap<Uuid, Vec<PublicTagResult>> = HashMap::new();
    for row in tag_rows {
        tags_by_post
            .entry(row.taggable_id)
            .or_default()
            .push(PublicTagResult {
                kind: row.tag.kind,
                value: row.tag.value,
                display_name: row.tag.display_name,
                color: row.tag.color,
            });
    }

    let mut urgent_notes_by_post: HashMap<Uuid, Vec<UrgentNoteInfo>> = HashMap::new();
    for (post_id, content, cta_text) in urgent_rows {
        urgent_notes_by_post
            .entry(post_id)
            .or_default()
            .push(UrgentNoteInfo { content, cta_text });
    }

    Ok((tags_by_post, urgent_notes_by_post))
}

// =============================================================================
// Helper: build a full PostResult for a single post (used by PostObject handlers)
// =============================================================================

async fn build_post_result(
    post_id: Uuid,
    show_private: bool,
    deps: &ServerDeps,
) -> ApiResult<PostResult> {
    let post = Post::find_by_id(PostId::from_uuid(post_id), &deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found".into()))?;

    let tags = if show_private {
        Tag::find_for_post(PostId::from_uuid(post_id), &deps.db_pool).await?
    } else {
        Tag::find_public_for_post_ids(&[post_id], &deps.db_pool)
            .await?
            .into_iter()
            .map(|t| t.tag)
            .collect()
    };

    // Resolve who submitted this post
    let submitted_by = post.submitted_by_id.map(|_| SubmittedByInfo {
        submitter_type: "member".to_string(),
    });

    // Load schedules
    let schedules = Schedule::find_for_post(post_id, &deps.db_pool).await?;

    // Load contacts
    let contacts = Contact::find_by_post(PostId::from_uuid(post_id), &deps.db_pool).await?;

    // Load organization through post_sources -> sources -> organizations
    let org_row = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT o.id, o.name
        FROM organizations o
        JOIN sources s ON s.organization_id = o.id
        JOIN post_sources ps ON ps.source_id = s.id
        WHERE ps.post_id = $1
        LIMIT 1
        "#,
    )
    .bind(post_id)
    .fetch_optional(&deps.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // Check for urgent notes
    let urgent_rows = Note::find_urgent_note_content_for_posts(&[post_id], &deps.db_pool)
        .await
        .unwrap_or_default();
    let urgent_note_texts: Vec<UrgentNoteInfo> = urgent_rows
        .into_iter()
        .map(|(_, content, cta_text)| UrgentNoteInfo { content, cta_text })
        .collect();

    let mut result = PostResult::from(post);
    if let Some((org_id, org_name)) = org_row {
        result.organization_id = Some(org_id);
        result.organization_name = Some(org_name);
    }
    result.has_urgent_notes = Some(!urgent_note_texts.is_empty());
    result.urgent_notes = if urgent_note_texts.is_empty() {
        None
    } else {
        Some(urgent_note_texts)
    };
    result.submitted_by = submitted_by;
    result.tags = Some(
        tags.into_iter()
            .map(|t| PostTagResult {
                id: t.id.into_uuid(),
                kind: t.kind,
                value: t.value,
                display_name: t.display_name,
                color: t.color,
            })
            .collect(),
    );
    result.schedules = Some(
        schedules
            .into_iter()
            .map(|s| PostScheduleResult {
                id: s.id.into_uuid(),
                day_of_week: s.day_of_week,
                opens_at: s.opens_at.map(|t| t.format("%H:%M").to_string()),
                closes_at: s.closes_at.map(|t| t.format("%H:%M").to_string()),
                timezone: s.timezone,
                notes: s.notes,
                rrule: s.rrule,
                dtstart: s.dtstart.map(|dt| dt.to_rfc3339()),
                dtend: s.dtend.map(|dt| dt.to_rfc3339()),
                is_all_day: s.is_all_day,
                duration_minutes: s.duration_minutes,
            })
            .collect(),
    );
    result.contacts = Some(
        contacts
            .into_iter()
            .map(|c| PostContactResult {
                id: c.id,
                contact_type: c.contact_type,
                contact_value: c.contact_value,
                contact_label: c.contact_label,
            })
            .collect(),
    );
    Ok(result)
}

// =============================================================================
// Handlers — Posts service (stateless, plural path: /Posts/...)
// =============================================================================

async fn list(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListPostsRequest>,
) -> ApiResult<Json<PostListResult>> {
    let deps = &state.deps;

    let filters = PostFilters {
        status: req.status.as_deref(),
        source_type: req.source_type.as_deref(),
        source_id: req.source_id,
        search: req.search.as_deref(),
        post_type: req.post_type.as_deref(),
        submission_type: req.submission_type.as_deref(),
        exclude_submission_type: req.exclude_submission_type.as_deref(),
        county_id: req.county_id,
        statewide_only: req.statewide_only.unwrap_or(false),
    };

    if let Some(ref zip_code) = req.zip_code {
        let radius = req.radius_miles.unwrap_or(25.0).min(100.0);
        let limit = req.first.unwrap_or(20);
        let offset = req.offset.unwrap_or(0);

        let (results, total_count, has_more) =
            activities::get_posts_near_zip(zip_code, radius, &filters, limit, offset, deps)
                .await?;

        let post_ids: Vec<Uuid> = results.iter().map(|r| r.id.into_uuid()).collect();
        let tag_rows = Tag::find_for_post_ids(&post_ids, &deps.db_pool).await?;

        let mut tags_by_post: HashMap<Uuid, Vec<PostTagResult>> = HashMap::new();
        for row in tag_rows {
            tags_by_post
                .entry(row.taggable_id)
                .or_default()
                .push(PostTagResult {
                    id: row.tag.id.into_uuid(),
                    kind: row.tag.kind,
                    value: row.tag.value,
                    display_name: row.tag.display_name,
                    color: row.tag.color,
                });
        }

        Ok(Json(PostListResult {
            posts: results
                .into_iter()
                .map(|pwd| {
                    let id = pwd.id.into_uuid();
                    PostResult {
                        id,
                        title: pwd.title,
                        body_raw: pwd.body_raw,
                        body_ast: None,
                        status: pwd.status,
                        post_type: pwd.post_type,
                        is_urgent: pwd.is_urgent,
                        pencil_mark: None,
                        location: pwd.location,
                        submission_type: pwd.submission_type,
                        created_at: pwd.created_at.to_rfc3339(),
                        updated_at: pwd.updated_at.to_rfc3339(),
                        published_at: pwd.published_at.map(|dt| dt.to_rfc3339()),
                        tags: Some(tags_by_post.remove(&id).unwrap_or_default()),
                        submitted_by: None,
                        schedules: Some(vec![]),
                        contacts: Some(vec![]),
                        organization_id: None,
                        organization_name: None,
                        has_urgent_notes: None,
                        urgent_notes: None,
                        zip_code: None,
                        weight: "medium".to_string(),
                        priority: 0,
                        body_heavy: None,
                        body_medium: None,
                        body_light: None,
                        distance_miles: Some(pwd.distance_miles),
                        latitude: None,
                        longitude: None,
                        revision_of_post_id: None,
                        translation_of_id: None,
                        duplicate_of_id: None,
                        source_language: "en".to_string(),
                    }
                })
                .collect(),
            total_count,
            has_next_page: has_more,
            has_previous_page: offset > 0,
        }))
    } else {
        let pagination_args = PaginationArgs {
            first: req.first,
            after: req.after,
            last: req.last,
            before: req.before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        let connection =
            activities::get_posts_paginated(&filters, &validated, deps).await?;

        let post_ids: Vec<Uuid> = connection.edges.iter().map(|e| e.node.id).collect();
        let tag_rows = Tag::find_for_post_ids(&post_ids, &deps.db_pool).await?;

        let mut tags_by_post: HashMap<Uuid, Vec<PostTagResult>> = HashMap::new();
        for row in tag_rows {
            tags_by_post
                .entry(row.taggable_id)
                .or_default()
                .push(PostTagResult {
                    id: row.tag.id.into_uuid(),
                    kind: row.tag.kind,
                    value: row.tag.value,
                    display_name: row.tag.display_name,
                    color: row.tag.color,
                });
        }

        Ok(Json(PostListResult {
            posts: connection
                .edges
                .into_iter()
                .map(|e| {
                    let id = e.node.id;
                    PostResult {
                        id,
                        title: e.node.title,
                        body_raw: e.node.body_raw,
                        body_ast: None,
                        status: e.node.status.to_string(),
                        post_type: e.node.post_type,
                        is_urgent: e.node.is_urgent,
                        pencil_mark: None,
                        location: e.node.location,
                        submission_type: e.node.submission_type,
                        created_at: e.node.created_at.to_rfc3339(),
                        updated_at: e.node.created_at.to_rfc3339(),
                        published_at: e.node.published_at.map(|dt| dt.to_rfc3339()),
                        tags: Some(tags_by_post.remove(&id).unwrap_or_default()),
                        submitted_by: None,
                        schedules: Some(vec![]),
                        contacts: Some(vec![]),
                        organization_id: None,
                        organization_name: None,
                        has_urgent_notes: None,
                        urgent_notes: None,
                        zip_code: None,
                        weight: "medium".to_string(),
                        priority: 0,
                        body_heavy: None,
                        body_medium: None,
                        body_light: None,
                        distance_miles: None,
                        latitude: None,
                        longitude: None,
                        revision_of_post_id: None,
                        translation_of_id: None,
                        duplicate_of_id: None,
                        source_language: "en".to_string(),
                    }
                })
                .collect(),
            total_count: connection.total_count,
            has_next_page: connection.page_info.has_next_page,
            has_previous_page: connection.page_info.has_previous_page,
        }))
    }
}

/// List posts eligible for a given edition, mirroring the layout engine's
/// county-matching logic (locationables, statewide tag, or no-location fallback)
/// with optional slotted/not_slotted filtering relative to the edition.
async fn list_for_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListPostsForEditionRequest>,
) -> ApiResult<Json<PostListResult>> {
    let deps = &state.deps;
    let edition = Edition::find_by_id(req.edition_id, &deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Edition not found: {}", req.edition_id)))?;

    let slotted_filter = req.slotted_filter.as_deref().unwrap_or("all");
    let limit = req.limit.unwrap_or(200);
    let offset = req.offset.unwrap_or(0);

    let posts = Post::find_for_edition(
        edition.county_id,
        edition.period_start,
        edition.id,
        slotted_filter,
        limit,
        offset,
        &deps.db_pool,
    )
    .await?;

    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
    let tag_rows = Tag::find_for_post_ids(&post_ids, &deps.db_pool).await?;

    let mut tags_by_post: HashMap<Uuid, Vec<PostTagResult>> = HashMap::new();
    for row in tag_rows {
        tags_by_post
            .entry(row.taggable_id)
            .or_default()
            .push(PostTagResult {
                id: row.tag.id.into_uuid(),
                kind: row.tag.kind,
                value: row.tag.value,
                display_name: row.tag.display_name,
                color: row.tag.color,
            });
    }

    let total_count = posts.len() as i32;
    let results: Vec<PostResult> = posts
        .into_iter()
        .map(|p| {
            let id = p.id.into_uuid();
            let mut pr = PostResult::from(p);
            pr.tags = Some(tags_by_post.remove(&id).unwrap_or_default());
            pr
        })
        .collect();

    Ok(Json(PostListResult {
        posts: results,
        total_count,
        has_next_page: false,
        has_previous_page: offset > 0,
    }))
}

async fn search_nearby(
    State(state): State<AppState>,
    Json(req): Json<NearbySearchRequest>,
) -> ApiResult<Json<NearbySearchResults>> {
    let deps = &state.deps;
    let radius = req.radius_miles.unwrap_or(25.0).min(100.0);
    let limit = req.limit.unwrap_or(50).min(200);

    let _center = ZipCode::find_by_code(&req.zip_code, &deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest(format!("Zip code '{}' not found", req.zip_code)))?;

    let results = Post::find_near_zip(&req.zip_code, radius, limit, &deps.db_pool).await?;

    Ok(Json(NearbySearchResults {
        results: results
            .into_iter()
            .map(|pwd| NearbyPostResult {
                post: PostResult {
                    id: pwd.id.into_uuid(),
                    title: pwd.title,
                    body_raw: pwd.body_raw,
                    body_ast: None,
                    status: pwd.status,
                    post_type: pwd.post_type,
                    is_urgent: pwd.is_urgent,
                    pencil_mark: None,
                    location: pwd.location,
                    submission_type: pwd.submission_type,
                    created_at: pwd.created_at.to_rfc3339(),
                    updated_at: pwd.updated_at.to_rfc3339(),
                    published_at: pwd.published_at.map(|dt| dt.to_rfc3339()),
                    zip_code: pwd.zip_code,
                    weight: "medium".to_string(),
                    priority: 0,
                    body_heavy: None,
                    body_medium: None,
                    body_light: None,
                    tags: None,
                    submitted_by: None,
                    schedules: Some(vec![]),
                    contacts: Some(vec![]),
                    organization_id: None,
                    organization_name: None,
                    has_urgent_notes: None,
                    urgent_notes: None,
                    distance_miles: None,
                    latitude: None,
                    longitude: None,
                    revision_of_post_id: None,
                    translation_of_id: None,
                    duplicate_of_id: None,
                    source_language: "en".to_string(),
                },
                distance_miles: pwd.distance_miles,
            })
            .collect(),
    }))
}

async fn submit(
    State(state): State<AppState>,
    user: OptionalUser,
    Json(req): Json<SubmitPostRequest>,
) -> ApiResult<Json<SubmitPostResult>> {
    use crate::domains::posts::data::types::ContactInfoInput;

    let contact_info = if req.contact_phone.is_some()
        || req.contact_email.is_some()
        || req.contact_website.is_some()
    {
        Some(ContactInfoInput {
            phone: req.contact_phone,
            email: req.contact_email,
            website: req.contact_website,
        })
    } else {
        None
    };

    let input = SubmitPostInput {
        title: req.title,
        body_raw: req.body_raw,
        contact_info,
        is_urgent: req.is_urgent,
        location: req.location,
    };

    let post_id = activities::submit_post(
        input,
        user.0.as_ref().map(|u| u.member_id.into_uuid()),
        &state.deps,
    )
    .await?;

    Ok(Json(SubmitPostResult {
        post_id: post_id.into_uuid(),
    }))
}

async fn list_pending_revisions(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<PendingRevisionsRequest>,
) -> ApiResult<Json<PendingRevisionsResult>> {
    let source_filter = match (req.source_type.as_deref(), req.source_id) {
        (Some(st), Some(sid)) => Some((st, sid)),
        _ => None,
    };

    let revisions =
        activities::revision_actions::get_pending_revisions(source_filter, &state.deps.db_pool)
            .await?;

    Ok(Json(PendingRevisionsResult {
        posts: revisions.into_iter().map(PostResult::from).collect(),
    }))
}

async fn list_reports(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListReportsRequest>,
) -> ApiResult<Json<ReportListResult>> {
    let limit = req.limit.unwrap_or(50) as i64;
    let offset = req.offset.unwrap_or(0) as i64;

    let reports: Vec<PostReportWithDetails> = match req.status.as_deref() {
        Some("pending") | None => {
            PostReportRecord::query_pending(limit, offset, &state.deps.db_pool).await
        }
        _ => PostReportRecord::query_all(limit, offset, &state.deps.db_pool).await,
    }
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(ReportListResult {
        reports: reports
            .into_iter()
            .map(|r| ReportDetailResult {
                id: r.id.into_uuid(),
                post_id: r.post_id.into_uuid(),
                reason: r.reason,
                category: r.category,
                status: r.status,
                reporter_email: None,
                resolution_notes: r.resolution_notes,
            })
            .collect(),
    }))
}

async fn upcoming_events(
    State(state): State<AppState>,
    Json(req): Json<UpcomingEventsRequest>,
) -> ApiResult<Json<UpcomingEventsResult>> {
    let limit = req.limit.unwrap_or(20).min(100) as usize;

    let events =
        activities::upcoming_events::get_upcoming_events(limit, &state.deps).await?;

    Ok(Json(UpcomingEventsResult {
        events: events
            .into_iter()
            .map(|e| EventPostResult {
                id: e.id,
                title: e.title,
                body_raw: e.body_raw,
                status: e.status.to_string(),
                location: e.location,
            })
            .collect(),
    }))
}

async fn schedules_for_entity(
    State(state): State<AppState>,
    Json(req): Json<SchedulesForEntityRequest>,
) -> ApiResult<Json<ScheduleListResult>> {
    let schedules = Schedule::find_for_entity(
        &req.schedulable_type,
        req.schedulable_id,
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(ScheduleListResult {
        schedules: schedules
            .into_iter()
            .map(|s| ScheduleDetailResult {
                id: s.id.into_uuid(),
                schedulable_type: s.schedulable_type,
                schedulable_id: s.schedulable_id,
                day_of_week: s.day_of_week,
                opens_at: s.opens_at.map(|t| t.to_string()),
                closes_at: s.closes_at.map(|t| t.to_string()),
                timezone: s.timezone,
                valid_from: s.valid_from.map(|d| d.to_string()),
                valid_to: s.valid_to.map(|d| d.to_string()),
                notes: s.notes,
                rrule: s.rrule,
            })
            .collect(),
    }))
}

async fn backfill_locations(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<BackfillLocationsRequest>,
) -> ApiResult<Json<BackfillLocationsResult>> {
    let batch_size = req.batch_size.unwrap_or(100).min(500) as i64;

    let r = activities::backfill::backfill_post_locations(batch_size, &state.deps).await?;

    Ok(Json(BackfillLocationsResult {
        processed: r.processed,
        failed: r.failed,
        remaining: r.remaining,
    }))
}

async fn list_by_organization(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListPostsByOrganizationRequest>,
) -> ApiResult<Json<PostListResult>> {
    let deps = &state.deps;

    let posts =
        Post::find_all_by_organization_id(req.organization_id, &deps.db_pool).await?;

    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
    let tag_rows = Tag::find_for_post_ids(&post_ids, &deps.db_pool).await?;

    let mut tags_by_post: HashMap<Uuid, Vec<PostTagResult>> = HashMap::new();
    for row in tag_rows {
        tags_by_post
            .entry(row.taggable_id)
            .or_default()
            .push(PostTagResult {
                id: row.tag.id.into_uuid(),
                kind: row.tag.kind,
                value: row.tag.value,
                display_name: row.tag.display_name,
                color: row.tag.color,
            });
    }

    let total_count = posts.len() as i32;

    Ok(Json(PostListResult {
        posts: posts
            .into_iter()
            .map(|p| {
                let id = p.id.into_uuid();
                PostResult {
                    id,
                    title: p.title,
                    body_raw: p.body_raw,
                    body_ast: p.body_ast,
                    status: p.status,
                    post_type: p.post_type,
                    is_urgent: p.is_urgent,
                    pencil_mark: p.pencil_mark.clone(),
                    location: p.location,
                    zip_code: p.zip_code,
                    submission_type: p.submission_type,
                    weight: p.weight,
                    priority: p.priority,
                    created_at: p.created_at.to_rfc3339(),
                    updated_at: p.updated_at.to_rfc3339(),
                    published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                    body_heavy: p.body_heavy,
                    body_medium: p.body_medium,
                    body_light: p.body_light,
                    tags: Some(tags_by_post.remove(&id).unwrap_or_default()),
                    submitted_by: None,
                    schedules: Some(vec![]),
                    contacts: Some(vec![]),
                    organization_id: None,
                    organization_name: None,
                    has_urgent_notes: None,
                    urgent_notes: None,
                    distance_miles: None,
                    latitude: None,
                    longitude: None,
                    revision_of_post_id: None,
                    translation_of_id: None,
                    duplicate_of_id: None,
                    source_language: "en".to_string(),
                }
            })
            .collect(),
        total_count,
        has_next_page: false,
        has_previous_page: false,
    }))
}

async fn public_list(
    State(state): State<AppState>,
    Json(req): Json<PublicListRequest>,
) -> ApiResult<Json<PublicListResult>> {
    let deps = &state.deps;
    let limit = req.limit.unwrap_or(50).min(200) as i64;
    let offset = req.offset.unwrap_or(0) as i64;
    let post_type = req.post_type.as_deref();
    let category: Option<&str> = None;

    let (post_items, total_count): (Vec<PublicPostResult>, i64) =
        if let Some(ref zip) = req.zip_code {
            let radius = req.radius_miles.unwrap_or(25.0).min(100.0);

            let nearby_posts = Post::find_public_filtered_near_zip(
                zip,
                radius,
                post_type,
                category,
                limit,
                offset,
                &deps.db_pool,
            )
            .await?;

            let count = Post::count_public_filtered_near_zip(
                zip, radius, post_type, category, &deps.db_pool,
            )
            .await?;

            let post_ids: Vec<Uuid> =
                nearby_posts.iter().map(|p| p.id.into_uuid()).collect();
            let (mut tags_by_post, mut urgent_notes_by_post) =
                load_tags_and_notes(&post_ids, deps).await?;
            let mut org_info =
                Post::find_org_info_for_posts(&post_ids, &deps.db_pool).await?;

            let items = nearby_posts
                .into_iter()
                .map(|p| {
                    let id = p.id.into_uuid();
                    let (org_id, org_name) = org_info
                        .remove(&id)
                        .map(|(oid, name)| (Some(oid), Some(name)))
                        .unwrap_or((None, None));
                    PublicPostResult {
                        id,
                        title: p.title,
                        body_raw: p.body_raw,
                        body_light: p.body_light,
                        location: p.location,
                        post_type: p.post_type,
                        is_urgent: p.is_urgent,
                        created_at: p.created_at.to_rfc3339(),
                        published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                        tags: tags_by_post.remove(&id).unwrap_or_default(),
                        urgent_notes: urgent_notes_by_post.remove(&id).unwrap_or_default(),
                        distance_miles: Some(p.distance_miles),
                        organization_id: org_id,
                        organization_name: org_name,
                    }
                })
                .collect();

            (items, count)
        } else {
            let posts = Post::find_public_filtered(
                post_type, category, limit, offset, &deps.db_pool,
            )
            .await?;

            let count =
                Post::count_public_filtered(post_type, category, &deps.db_pool).await?;

            let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
            let (mut tags_by_post, mut urgent_notes_by_post) =
                load_tags_and_notes(&post_ids, deps).await?;
            let mut org_info =
                Post::find_org_info_for_posts(&post_ids, &deps.db_pool).await?;

            let items = posts
                .into_iter()
                .map(|p| {
                    let id = p.id.into_uuid();
                    let (org_id, org_name) = org_info
                        .remove(&id)
                        .map(|(oid, name)| (Some(oid), Some(name)))
                        .unwrap_or((None, None));
                    PublicPostResult {
                        id,
                        title: p.title,
                        body_raw: p.body_raw,
                        body_light: p.body_light,
                        location: p.location,
                        post_type: p.post_type,
                        is_urgent: p.is_urgent,
                        created_at: p.created_at.to_rfc3339(),
                        published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                        tags: tags_by_post.remove(&id).unwrap_or_default(),
                        urgent_notes: urgent_notes_by_post.remove(&id).unwrap_or_default(),
                        distance_miles: None,
                        organization_id: org_id,
                        organization_name: org_name,
                    }
                })
                .collect();

            (items, count)
        };

    Ok(Json(PublicListResult {
        posts: post_items,
        total_count: total_count as i32,
    }))
}

async fn public_filters(
    State(state): State<AppState>,
    Json(_req): Json<PublicFiltersRequest>,
) -> ApiResult<Json<PublicFiltersResult>> {
    let post_types = Tag::find_post_types(&state.deps.db_pool).await?;

    Ok(Json(PublicFiltersResult {
        post_types: post_types
            .into_iter()
            .map(|t| PostTypeOption {
                value: t.value,
                display_name: t.display_name.unwrap_or_default(),
                description: t.description,
                color: t.color,
                emoji: t.emoji,
            })
            .collect(),
    }))
}

async fn expire_stale_posts(
    State(state): State<AppState>,
    Json(_req): Json<ExpireStalePostsRequest>,
) -> ApiResult<Json<ExpireStalePostsResult>> {
    let expired_count =
        activities::expire_scheduled_posts::expire_scheduled_posts(&state.deps).await?;

    Ok(Json(ExpireStalePostsResult { expired_count }))
}

async fn stats(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<PostStatsRequest>,
) -> ApiResult<Json<PostStatsResult>> {
    let rows = Post::stats_by_status(req.status.as_deref(), &state.deps.db_pool).await?;

    let mut total: i64 = 0;
    let mut stories: i64 = 0;
    let mut notices: i64 = 0;
    let mut exchanges: i64 = 0;
    let mut events: i64 = 0;
    let mut spotlights: i64 = 0;
    let mut references: i64 = 0;
    let mut ingested: i64 = 0;
    let mut user_submitted: i64 = 0;

    for (post_type, submission_type, count) in &rows {
        total += count;

        match post_type.as_deref() {
            Some("story") => stories += count,
            Some("notice") => notices += count,
            Some("exchange") => exchanges += count,
            Some("event") => events += count,
            Some("spotlight") => spotlights += count,
            Some("reference") => references += count,
            _ => {}
        }

        match submission_type.as_deref() {
            Some("ingested") => ingested += count,
            _ => user_submitted += count,
        }
    }

    Ok(Json(PostStatsResult {
        total,
        stories,
        notices,
        exchanges,
        events,
        spotlights,
        references,
        user_submitted,
        ingested,
    }))
}

async fn create_post(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<CreatePostRequest>,
) -> ApiResult<Json<PostResult>> {
    let post = activities::admin_create_post(
        req.title,
        req.body_raw,
        req.post_type,
        req.weight,
        req.priority,
        req.is_urgent,
        req.location,
        user.0.member_id.into_uuid(),
        &state.deps,
    )
    .await?;

    Ok(Json(PostResult::from(post)))
}

// =============================================================================
// Handlers — Post virtual object (keyed, singular path: /Post/{id}/...)
// =============================================================================

async fn get_post(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: OptionalUser,
    Json(req): Json<GetPostRequest>,
) -> ApiResult<Json<PostResult>> {
    let is_admin = user.0.as_ref().map(|u| u.is_admin).unwrap_or(false);

    // Non-admins can only see active, non-deleted posts
    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found".into()))?;

    if !is_admin && (post.status != "active" || post.deleted_at.is_some()) {
        return Err(ApiError::NotFound("Post not found".into()));
    }

    let include_private = is_admin && req.show_private;
    build_post_result(post_id, include_private, &state.deps)
        .await
        .map(Json)
}

async fn get_related_posts(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: OptionalUser,
) -> ApiResult<Json<Vec<PublicPostResult>>> {
    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found".into()))?;

    let related = Post::find_related(
        post.id,
        post.zip_code.as_deref(),
        &post.post_type,
        &state.deps.db_pool,
    )
    .await?;

    let post_ids: Vec<Uuid> = related.iter().map(|p| p.id.into_uuid()).collect();
    let (mut tags_by_post, _urgent_notes_by_post) =
        load_tags_and_notes(&post_ids, &state.deps).await?;

    let results = related
        .into_iter()
        .map(|p| {
            let id = p.id.into_uuid();
            PublicPostResult {
                id,
                title: p.title,
                body_raw: p.body_raw,
                body_light: p.body_light,
                location: p.location,
                post_type: p.post_type,
                is_urgent: p.is_urgent,
                created_at: p.created_at.to_rfc3339(),
                published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                tags: tags_by_post.remove(&id).unwrap_or_default(),
                urgent_notes: vec![],
                distance_miles: None,
                organization_id: None,
                organization_name: None,
            }
        })
        .collect();

    Ok(Json(results))
}

async fn approve(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(_req): Json<ApprovePostRequest>,
) -> ApiResult<Json<PostResult>> {
    activities::approve_post(
        post_id,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        &state.deps,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after approve".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn edit_and_approve(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(req): Json<EditApproveRequest>,
) -> ApiResult<Json<PostResult>> {
    use crate::domains::posts::data::types::EditPostInput;

    let edit_input = EditPostInput {
        title: req.title,
        body_raw: req.body_raw,
        is_urgent: req.is_urgent,
        location: req.location,
    };

    activities::edit_and_approve_post(
        post_id,
        edit_input,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        &state.deps,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after edit_and_approve".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn reject(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(req): Json<RejectPostRequest>,
) -> ApiResult<Json<PostResult>> {
    activities::reject_post(
        post_id,
        req.reason,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        &state.deps,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after reject".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn delete(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<()>> {
    activities::delete_post(
        post_id,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        &state.deps,
    )
    .await?;

    Ok(Json(()))
}

async fn archive(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostResult>> {
    activities::archive_post(
        post_id,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        &state.deps,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after archive".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn reactivate(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostResult>> {
    Post::update_status(PostId::from_uuid(post_id), "active", &state.deps.db_pool)
        .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after reactivate".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn expire(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostResult>> {
    activities::expire_post(
        post_id,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        &state.deps,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after expire".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn report(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: OptionalUser,
    Json(req): Json<ReportPostRequest>,
) -> ApiResult<Json<()>> {
    activities::report_post(
        post_id,
        user.0.as_ref().map(|u| u.member_id.into_uuid()),
        req.reporter_email,
        req.reason,
        req.category,
        &state.deps,
    )
    .await?;

    Ok(Json(()))
}

async fn resolve_report(
    State(state): State<AppState>,
    Path(_post_id): Path<Uuid>,
    user: AdminUser,
    Json(req): Json<ResolveReportRequest>,
) -> ApiResult<Json<()>> {
    activities::resolve_report(
        req.report_id,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        req.resolution_notes,
        req.action_taken,
        &state.deps,
    )
    .await?;

    Ok(Json(()))
}

async fn dismiss_report(
    State(state): State<AppState>,
    Path(_post_id): Path<Uuid>,
    user: AdminUser,
    Json(req): Json<DismissReportRequest>,
) -> ApiResult<Json<()>> {
    activities::dismiss_report(
        req.report_id,
        user.0.member_id.into_uuid(),
        user.0.is_admin,
        req.resolution_notes,
        &state.deps,
    )
    .await?;

    Ok(Json(()))
}

async fn update_tags(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpdateTagsRequest>,
) -> ApiResult<Json<PostResult>> {
    let tags: Vec<TagInput> = req
        .tags
        .into_iter()
        .map(|t| TagInput {
            kind: t.kind,
            value: t.value,
        })
        .collect();

    activities::tags::update_post_tags(post_id, tags, &state.deps.db_pool).await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after update_tags".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn add_tag(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<AddTagRequest>,
) -> ApiResult<Json<()>> {
    activities::tags::add_post_tag(
        post_id,
        req.tag_kind,
        req.tag_value,
        req.display_name,
        req.color,
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(()))
}

async fn remove_tag(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<RemoveTagRequest>,
) -> ApiResult<Json<()>> {
    activities::tags::remove_post_tag(post_id, req.tag_id, &state.deps.db_pool).await?;

    Ok(Json(()))
}

async fn add_contact(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<AddContactRequest>,
) -> ApiResult<Json<()>> {
    activities::contacts::add_post_contact(
        post_id,
        &req.contact_type,
        req.contact_value,
        req.contact_label,
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(()))
}

async fn remove_contact(
    State(state): State<AppState>,
    Path(_post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<RemoveContactRequest>,
) -> ApiResult<Json<()>> {
    activities::contacts::remove_post_contact(req.contact_id, &state.deps.db_pool).await?;

    Ok(Json(()))
}

async fn add_schedule(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<AddScheduleRequest>,
) -> ApiResult<Json<ScheduleResult>> {
    let params = ScheduleParams {
        dtstart: req.dtstart,
        dtend: req.dtend,
        rrule: req.rrule,
        exdates: req.exdates,
        opens_at: req.opens_at,
        closes_at: req.closes_at,
        day_of_week: req.day_of_week,
        timezone: req.timezone,
        is_all_day: req.is_all_day,
        duration_minutes: req.duration_minutes,
        notes: req.notes,
    };

    activities::schedule::add_post_schedule(post_id, params, &state.deps).await?;

    let schedules = Schedule::find_for_post(post_id, &state.deps.db_pool).await?;
    let schedule = schedules
        .last()
        .ok_or_else(|| ApiError::NotFound("Schedule not found after add".into()))?;

    Ok(Json(ScheduleResult {
        id: schedule.id.into_uuid(),
        schedulable_type: schedule.schedulable_type.clone(),
        schedulable_id: schedule.schedulable_id,
    }))
}

async fn update_schedule(
    State(state): State<AppState>,
    Path(_post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpdateScheduleRequest>,
) -> ApiResult<Json<ScheduleResult>> {
    let schedule_id = ScheduleId::from_uuid(req.schedule_id);
    let params = ScheduleParams {
        dtstart: req.dtstart,
        dtend: req.dtend,
        rrule: req.rrule,
        exdates: req.exdates,
        opens_at: req.opens_at,
        closes_at: req.closes_at,
        day_of_week: req.day_of_week,
        timezone: req.timezone,
        is_all_day: req.is_all_day,
        duration_minutes: req.duration_minutes,
        notes: req.notes,
    };

    activities::schedule::update_schedule(schedule_id, params, &state.deps).await?;

    let schedule = Schedule::find_by_id(schedule_id, &state.deps.db_pool).await?;

    Ok(Json(ScheduleResult {
        id: schedule.id.into_uuid(),
        schedulable_type: schedule.schedulable_type,
        schedulable_id: schedule.schedulable_id,
    }))
}

async fn delete_schedule(
    State(state): State<AppState>,
    Path(_post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<DeleteScheduleRequest>,
) -> ApiResult<Json<()>> {
    let schedule_id = ScheduleId::from_uuid(req.schedule_id);
    activities::schedule::delete_schedule(schedule_id, &state.deps).await?;
    Ok(Json(()))
}

async fn track_view(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<()>> {
    activities::track_post_view(post_id, &state.deps).await?;
    Ok(Json(()))
}

async fn track_click(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<()>> {
    activities::track_post_click(post_id, &state.deps).await?;
    Ok(Json(()))
}

async fn approve_revision(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostResult>> {
    activities::revision_actions::approve_revision(
        PostId::from_uuid(post_id),
        &state.deps.db_pool,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Revision not found after approve".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn reject_revision(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<()>> {
    activities::revision_actions::reject_revision(
        PostId::from_uuid(post_id),
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(()))
}

async fn regenerate(
    State(_state): State<AppState>,
    Path(_post_id): Path<Uuid>,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostResult>> {
    Err(ApiError::BadRequest(
        "Post regeneration requires the crawling pipeline which has been removed".into(),
    ))
}

async fn update_content(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    user: AdminUser,
    Json(req): Json<UpdatePostContentRequest>,
) -> ApiResult<Json<PostResult>> {
    activities::admin_update_post(
        post_id,
        req.title,
        req.body_raw,
        req.body_ast,
        req.post_type,
        req.weight,
        req.priority,
        req.is_urgent,
        req.location,
        req.zip_code,
        user.0.member_id.into_uuid(),
        &state.deps,
    )
    .await?;

    let post = Post::find_by_id(PostId::from_uuid(post_id), &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("Post not found after update_content".into()))?;

    Ok(Json(PostResult::from(post)))
}

async fn get_reports(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostReportListResult>> {
    let reports =
        PostReportRecord::query_for_post(PostId::from_uuid(post_id), &state.deps.db_pool).await?;

    Ok(Json(PostReportListResult {
        reports: reports
            .into_iter()
            .map(|r| ReportResult {
                id: r.id.into_uuid(),
                post_id: r.post_id.into_uuid(),
                reason: r.reason,
                category: r.category,
                status: r.status,
            })
            .collect(),
    }))
}

async fn get_revision(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<OptionalPostResult>> {
    let revision = activities::revision_actions::get_revision_for_post(
        PostId::from_uuid(post_id),
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(OptionalPostResult {
        post: revision.map(PostResult::from),
    }))
}

// =============================================================================
// Field group read handler
// =============================================================================

#[derive(Debug, Serialize)]
pub struct PostFieldGroupsResult {
    pub media: Vec<PostMediaRecord>,
    pub items: Vec<crate::domains::posts::models::PostItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<PostPersonRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<PostLinkRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_attribution: Option<PostSourceAttr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PostMetaRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<PostDatetimeRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_status: Option<PostStatusRecord>,
    pub schedule: Vec<crate::domains::posts::models::PostScheduleEntry>,
}

async fn get_field_groups(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: OptionalUser,
) -> ApiResult<Json<PostFieldGroupsResult>> {
    let pool = &state.deps.db_pool;
    let ids = &[post_id];

    let media = PostMediaRecord::find_by_post_ids(ids, pool).await?;
    let items = crate::domains::posts::models::PostItem::find_by_post_ids(ids, pool).await?;
    let persons = PostPersonRecord::find_by_post_ids(ids, pool).await?;
    let links = PostLinkRecord::find_by_post_ids(ids, pool).await?;
    let source_attrs = PostSourceAttr::find_by_post_ids(ids, pool).await?;
    let metas = PostMetaRecord::find_by_post_ids(ids, pool).await?;
    let datetimes = PostDatetimeRecord::find_by_post_ids(ids, pool).await?;
    let statuses = PostStatusRecord::find_by_post_ids(ids, pool).await?;
    let schedule = crate::domains::posts::models::PostScheduleEntry::find_by_post_ids(ids, pool).await?;

    Ok(Json(PostFieldGroupsResult {
        media,
        items,
        person: persons.into_iter().next(),
        link: links.into_iter().next(),
        source_attribution: source_attrs.into_iter().next(),
        meta: metas.into_iter().next(),
        datetime: datetimes.into_iter().next(),
        post_status: statuses.into_iter().next(),
        schedule,
    }))
}

// =============================================================================
// Field group upsert handlers
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct UpsertPostMediaRequest {
    pub image_url: Option<String>,
    pub caption: Option<String>,
    pub credit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FieldGroupResult {
    pub success: bool,
}

async fn upsert_post_media(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostMediaRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    PostMediaRecord::upsert_primary(
        post_id,
        req.image_url.as_deref(),
        req.caption.as_deref(),
        req.credit.as_deref(),
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostMetaRequest {
    pub kicker: Option<String>,
    pub byline: Option<String>,
    pub deck: Option<String>,
    pub updated: Option<String>,
}

async fn upsert_post_meta(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostMetaRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    PostMetaRecord::upsert(
        post_id,
        req.kicker.as_deref(),
        req.byline.as_deref(),
        req.deck.as_deref(),
        req.updated.as_deref(),
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostPersonRequest {
    pub name: Option<String>,
    pub role: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub quote: Option<String>,
}

async fn upsert_post_person(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostPersonRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    PostPersonRecord::upsert(
        post_id,
        req.name.as_deref(),
        req.role.as_deref(),
        req.bio.as_deref(),
        req.photo_url.as_deref(),
        req.quote.as_deref(),
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostLinkRequest {
    pub label: Option<String>,
    pub url: Option<String>,
    pub deadline: Option<String>, // ISO date string
}

async fn upsert_post_link(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostLinkRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    let deadline = req
        .deadline
        .as_deref()
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    PostLinkRecord::upsert(
        post_id,
        req.label.as_deref(),
        req.url.as_deref(),
        deadline,
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostSourceAttrRequest {
    pub source_name: Option<String>,
    pub attribution: Option<String>,
}

async fn upsert_post_source_attr(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostSourceAttrRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    PostSourceAttr::upsert(
        post_id,
        req.source_name.as_deref(),
        req.attribution.as_deref(),
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostDatetimeRequest {
    pub start_at: Option<String>, // ISO datetime
    pub end_at: Option<String>,   // ISO datetime
    pub cost: Option<String>,
    pub recurring: Option<bool>,
}

async fn upsert_post_datetime(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostDatetimeRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    let start_at = req
        .start_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    let end_at = req
        .end_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    PostDatetimeRecord::upsert(
        post_id,
        start_at,
        end_at,
        req.cost.as_deref(),
        req.recurring.unwrap_or(false),
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostStatusRequest {
    pub state: Option<String>,
    pub verified: Option<String>,
}

async fn upsert_post_status(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    _user: AdminUser,
    Json(req): Json<UpsertPostStatusRequest>,
) -> ApiResult<Json<FieldGroupResult>> {
    let pool = &state.deps.db_pool;
    PostStatusRecord::upsert(
        post_id,
        req.state.as_deref(),
        req.verified.as_deref(),
        pool,
    )
    .await?;
    Ok(Json(FieldGroupResult { success: true }))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // --- Posts service (stateless, plural) ---
        .route("/Posts/list", post(list))
        .route("/Posts/list_for_edition", post(list_for_edition))
        .route("/Posts/search_nearby", post(search_nearby))
        .route("/Posts/submit", post(submit))
        .route("/Posts/list_pending_revisions", post(list_pending_revisions))
        .route("/Posts/list_reports", post(list_reports))
        .route("/Posts/upcoming_events", post(upcoming_events))
        .route("/Posts/schedules_for_entity", post(schedules_for_entity))
        .route("/Posts/backfill_locations", post(backfill_locations))
        .route("/Posts/list_by_organization", post(list_by_organization))
        .route("/Posts/public_list", post(public_list))
        .route("/Posts/public_filters", post(public_filters))
        .route("/Posts/expire_stale_posts", post(expire_stale_posts))
        .route("/Posts/stats", post(stats))
        .route("/Posts/create_post", post(create_post))
        // --- Post object (keyed, singular) ---
        .route("/Post/{id}/get", post(get_post))
        .route("/Post/{id}/approve", post(approve))
        .route("/Post/{id}/edit_and_approve", post(edit_and_approve))
        .route("/Post/{id}/reject", post(reject))
        .route("/Post/{id}/delete", post(delete))
        .route("/Post/{id}/archive", post(archive))
        .route("/Post/{id}/reactivate", post(reactivate))
        .route("/Post/{id}/expire", post(expire))
        .route("/Post/{id}/report", post(report))
        .route("/Post/{id}/resolve_report", post(resolve_report))
        .route("/Post/{id}/dismiss_report", post(dismiss_report))
        .route("/Post/{id}/update_tags", post(update_tags))
        .route("/Post/{id}/add_tag", post(add_tag))
        .route("/Post/{id}/remove_tag", post(remove_tag))
        .route("/Post/{id}/add_contact", post(add_contact))
        .route("/Post/{id}/remove_contact", post(remove_contact))
        .route("/Post/{id}/add_schedule", post(add_schedule))
        .route("/Post/{id}/update_schedule", post(update_schedule))
        .route("/Post/{id}/delete_schedule", post(delete_schedule))
        .route("/Post/{id}/track_view", post(track_view))
        .route("/Post/{id}/track_click", post(track_click))
        .route("/Post/{id}/approve_revision", post(approve_revision))
        .route("/Post/{id}/reject_revision", post(reject_revision))
        .route("/Post/{id}/regenerate", post(regenerate))
        .route("/Post/{id}/update_content", post(update_content))
        .route("/Post/{id}/get_reports", post(get_reports))
        .route("/Post/{id}/get_revision", post(get_revision))
        // Field groups
        .route("/Post/{id}/related", post(get_related_posts))
        // Field groups
        .route("/Post/{id}/field_groups", post(get_field_groups))
        .route("/Post/{id}/upsert_media", post(upsert_post_media))
        .route("/Post/{id}/upsert_meta", post(upsert_post_meta))
        .route("/Post/{id}/upsert_person", post(upsert_post_person))
        .route("/Post/{id}/upsert_link", post(upsert_post_link))
        .route("/Post/{id}/upsert_source_attr", post(upsert_post_source_attr))
        .route("/Post/{id}/upsert_datetime", post(upsert_post_datetime))
        .route("/Post/{id}/upsert_status", post(upsert_post_status))
}
