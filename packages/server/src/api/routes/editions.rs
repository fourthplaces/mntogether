use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::collections::HashMap;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::routes::posts::{load_tags_and_notes, PublicTagResult, UrgentNoteInfo};
use crate::api::state::AppState;
use crate::domains::editions::activities;
use crate::domains::editions::models::county::County;
use crate::domains::editions::models::edition::{Edition, EditionFilters};
use crate::domains::editions::models::edition_row::EditionRow;
use crate::domains::editions::models::edition_section::EditionSection;
use crate::domains::editions::models::edition_slot::EditionSlot;
use crate::domains::widgets::Widget;
use crate::domains::editions::models::post_template_config::PostTemplateConfig;
use crate::domains::editions::models::row_template_config::RowTemplateConfig;
use crate::domains::editions::models::row_template_slot::RowTemplateSlot;
use crate::domains::contacts::models::contact::Contact;
use crate::domains::posts::models::post::Post;
use crate::domains::posts::models::{
    PostDatetimeRecord, PostItem, PostLinkRecord, PostMediaRecord, PostMetaRecord,
    PostPersonRecord, PostScheduleEntry, PostSourceAttr, PostStatusRecord,
};

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct EmptyRequest {}

#[derive(Debug, Deserialize)]
pub struct GetCountyRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListEditionsRequest {
    pub county_id: Option<Uuid>,
    pub status: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetEditionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CurrentEditionRequest {
    pub county_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateEditionRequest {
    pub county_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateEditionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PublishEditionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveEditionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct BatchGenerateRequest {
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEditionRowRequest {
    pub row_id: Uuid,
    pub row_template_slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderRowsRequest {
    pub edition_id: Uuid,
    pub row_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct RemovePostFromEditionRequest {
    pub slot_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ChangeSlotTemplateRequest {
    pub slot_id: Uuid,
    pub post_template: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveSlotRequest {
    pub slot_id: Uuid,
    pub target_row_id: Uuid,
    pub slot_index: i32,
}

#[derive(Debug, Deserialize)]
pub struct AddPostToEditionRequest {
    pub edition_row_id: Uuid,
    pub post_id: Uuid,
    pub post_template: String,
    pub slot_index: i32,
}

#[derive(Debug, Deserialize)]
pub struct AddEditionRowRequest {
    pub edition_id: Uuid,
    pub row_template_slug: String,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct DeleteEditionRowRequest {
    pub row_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReviewEditionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ApproveEditionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct BatchApproveEditionsRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct BatchPublishEditionsRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct EditionKanbanStatsRequest {
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Deserialize)]
pub struct AddWidgetToEditionRequest {
    pub edition_row_id: Uuid,
    pub widget_id: Uuid,
    pub slot_index: i32,
}

// Section CRUD requests
#[derive(Debug, Deserialize)]
pub struct AddSectionRequest {
    pub edition_id: Uuid,
    pub title: String,
    pub subtitle: Option<String>,
    pub topic_slug: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSectionRequest {
    pub id: Uuid,
    pub title: Option<String>,
    pub subtitle: Option<Option<String>>,
    pub topic_slug: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderSectionsRequest {
    pub edition_id: Uuid,
    pub section_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteSectionRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AssignRowToSectionRequest {
    pub row_id: Uuid,
    pub section_id: Option<Uuid>,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct CountyResult {
    pub id: Uuid,
    pub fips_code: String,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct CountyListResult {
    pub counties: Vec<CountyResult>,
}

#[derive(Debug, Serialize)]
pub struct EditionResult {
    pub id: Uuid,
    pub county_id: Uuid,
    pub title: Option<String>,
    pub period_start: String,
    pub period_end: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct EditionListResult {
    pub editions: Vec<EditionResult>,
    pub total_count: i64,
}

#[derive(Debug, Serialize)]
pub struct EditionDetailResult {
    pub edition: EditionResult,
    pub rows: Vec<EditionRowResult>,
    pub sections: Vec<EditionSectionResult>,
}

#[derive(Debug, Serialize)]
pub struct EditionRowResult {
    pub id: Uuid,
    pub row_template_slug: String,
    pub layout_variant: String,
    pub row_template_id: Uuid,
    pub row_template_display_name: String,
    pub row_template_description: Option<String>,
    pub row_template_slots: Vec<RowTemplateSlotResult>,
    pub sort_order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_id: Option<Uuid>,
    pub slots: Vec<EditionSlotResult>,
}

#[derive(Debug, Serialize)]
pub struct EditionSlotResult {
    pub id: Uuid,
    pub kind: String,
    pub slot_index: i32,
    // Post fields (present when kind='post')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_post_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_status: Option<String>,
    // Widget fields (present when kind='widget')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_authoring_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct RowTemplateResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub layout_variant: String,
    pub slots: Vec<RowTemplateSlotResult>,
}

#[derive(Debug, Serialize)]
pub struct RowTemplateSlotResult {
    pub slot_index: i32,
    pub weight: String,
    pub count: i32,
    pub accepts: Option<Vec<String>>,
    pub post_template_slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RowTemplateListResult {
    pub templates: Vec<RowTemplateResult>,
}

#[derive(Debug, Serialize)]
pub struct PostTemplateResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub compatible_types: Vec<String>,
    pub body_target: i32,
    pub body_max: i32,
    pub title_max: i32,
    pub weight: String,
}

#[derive(Debug, Serialize)]
pub struct PostTemplateListResult {
    pub templates: Vec<PostTemplateResult>,
}

#[derive(Debug, Serialize)]
pub struct BatchGenerateEditionsResult {
    pub created: i32,
    pub regenerated: i32,
    pub skipped: i32,
    pub failed: i32,
    pub total_counties: i32,
}

#[derive(Debug, Serialize)]
pub struct ReorderRowsResult {
    pub rows: Vec<EditionRowResult>,
}

#[derive(Debug, Serialize)]
pub struct BatchEditionsResult {
    pub succeeded: i32,
    pub failed: i32,
}

#[derive(Debug, Serialize)]
pub struct EditionKanbanStatsResult {
    pub draft: i32,
    pub in_review: i32,
    pub approved: i32,
    pub published: i32,
}

// =============================================================================
// Public broadsheet result types (unauthenticated, full post data)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct PublicBroadsheetResult {
    pub edition: EditionResult,
    pub county: CountyResult,
    pub rows: Vec<PublicBroadsheetRowResult>,
    pub sections: Vec<EditionSectionResult>,
}

#[derive(Debug, Serialize)]
pub struct PublicBroadsheetRowResult {
    pub row_template_slug: String,
    pub layout_variant: String,
    pub sort_order: i32,
    pub section_id: Option<Uuid>,
    pub slots: Vec<PublicBroadsheetSlotResult>,
}

#[derive(Debug, Serialize)]
pub struct PublicBroadsheetSlotResult {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_template: Option<String>,
    pub slot_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<PublicBroadsheetPostResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget: Option<PublicBroadsheetWidgetResult>,
}

#[derive(Debug, Serialize)]
pub struct PublicBroadsheetWidgetResult {
    pub id: Uuid,
    pub widget_type: String,
    pub authoring_mode: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PublicBroadsheetPostResult {
    pub id: Uuid,
    pub title: String,
    pub body_raw: String,
    pub post_type: String,
    pub weight: String,
    pub is_urgent: bool,
    pub location: Option<String>,
    pub organization_name: Option<String>,
    pub published_at: Option<String>,
    pub tags: Vec<PublicTagResult>,
    pub contacts: Vec<BroadsheetContactResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub urgent_notes: Vec<UrgentNoteInfo>,
    // Weight-specific body text from Root Signal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_heavy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_light: Option<String>,
    // Field groups
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<BroadsheetMediaResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<BroadsheetItemResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<BroadsheetPersonResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<BroadsheetLinkResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_attribution: Option<BroadsheetSourceAttributionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<BroadsheetMetaResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<BroadsheetDatetimeResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_status: Option<BroadsheetStatusResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schedule: Vec<BroadsheetScheduleEntryResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BroadsheetContactResult {
    pub contact_type: String,
    pub contact_value: String,
    pub contact_label: Option<String>,
}

// =============================================================================
// Field group result types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct BroadsheetMediaResult {
    pub image_url: Option<String>,
    pub caption: Option<String>,
    pub credit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetItemResult {
    pub name: String,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetPersonResult {
    pub name: Option<String>,
    pub role: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub quote: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetLinkResult {
    pub label: Option<String>,
    pub url: Option<String>,
    pub deadline: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetSourceAttributionResult {
    pub source_name: Option<String>,
    pub attribution: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetMetaResult {
    pub kicker: Option<String>,
    pub byline: Option<String>,
    pub timestamp: Option<String>,
    pub updated: Option<String>,
    pub deck: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetDatetimeResult {
    pub start: Option<String>,
    pub end: Option<String>,
    pub cost: Option<String>,
    pub recurring: bool,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetStatusResult {
    pub state: Option<String>,
    pub verified: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BroadsheetScheduleEntryResult {
    pub day: String,
    pub opens: String,
    pub closes: String,
}

#[derive(Debug, Serialize)]
pub struct EditionSectionResult {
    pub id: Uuid,
    pub edition_id: Uuid,
    pub title: String,
    pub subtitle: Option<String>,
    pub topic_slug: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
}

fn section_to_result(s: &EditionSection) -> EditionSectionResult {
    EditionSectionResult {
        id: s.id,
        edition_id: s.edition_id,
        title: s.title.clone(),
        subtitle: s.subtitle.clone(),
        topic_slug: s.topic_slug.clone(),
        sort_order: s.sort_order,
        created_at: s.created_at.to_rfc3339(),
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn edition_to_result(e: &Edition) -> EditionResult {
    EditionResult {
        id: e.id,
        county_id: e.county_id,
        title: e.title.clone(),
        period_start: e.period_start.to_string(),
        period_end: e.period_end.to_string(),
        status: e.status.clone(),
        published_at: e.published_at.map(|t| t.to_rfc3339()),
        created_at: e.created_at.to_rfc3339(),
        row_count: None,
    }
}

async fn load_edition_detail(
    edition: &Edition,
    pool: &sqlx::PgPool,
) -> ApiResult<EditionDetailResult> {
    let rows = EditionRow::find_by_edition(edition.id, pool).await?;

    // Load all templates + slots upfront (2 queries total, avoids N+1)
    let all_templates = RowTemplateConfig::find_all(pool).await?;
    let all_template_slots = RowTemplateSlot::find_all(pool).await?;

    let mut row_results = Vec::new();
    for row in &rows {
        let template = all_templates
            .iter()
            .find(|t| t.id == row.row_template_config_id);
        let template_slot_results: Vec<RowTemplateSlotResult> = all_template_slots
            .iter()
            .filter(|s| s.row_template_config_id == row.row_template_config_id)
            .map(|s| RowTemplateSlotResult {
                slot_index: s.slot_index,
                weight: s.weight.clone(),
                count: s.count,
                accepts: s.accepts.clone(),
                post_template_slug: s.post_template_slug.clone(),
            })
            .collect();

        let slots = EditionSlot::find_by_row_with_content(row.id, pool).await?;

        row_results.push(EditionRowResult {
            id: row.id,
            row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
            layout_variant: template.map(|t| t.layout_variant.clone()).unwrap_or_else(|| "full".to_string()),
            row_template_id: row.row_template_config_id,
            row_template_display_name: template
                .map(|t| t.display_name.clone())
                .unwrap_or_default(),
            row_template_description: template.and_then(|t| t.description.clone()),
            row_template_slots: template_slot_results,
            sort_order: row.sort_order,
            section_id: row.section_id,
            slots: slots
                .iter()
                .map(|s| EditionSlotResult {
                    id: s.id,
                    kind: s.kind.clone(),
                    slot_index: s.slot_index,
                    post_id: s.post_id,
                    post_template: s.post_template.clone(),
                    post_title: s.post_title.clone(),
                    post_post_type: s.post_post_type.clone(),
                    post_weight: s.post_weight.clone(),
                    post_status: s.post_status.clone(),
                    widget_id: s.widget_id,
                    widget_type: s.widget_type.clone(),
                    widget_authoring_mode: s.widget_authoring_mode.clone(),
                    widget_data: s.widget_data.clone(),
                })
                .collect(),
        });
    }

    let sections = EditionSection::find_by_edition(edition.id, pool).await?;

    Ok(EditionDetailResult {
        edition: edition_to_result(edition),
        rows: row_results,
        sections: sections.iter().map(section_to_result).collect(),
    })
}

/// Build an `EditionRowResult` from a row, loading its template + slots + edition slots.
async fn build_row_result(
    row: &EditionRow,
    pool: &sqlx::PgPool,
) -> ApiResult<EditionRowResult> {
    let template = RowTemplateConfig::find_by_id(row.row_template_config_id, pool).await?;

    let template_slots =
        RowTemplateSlot::find_by_template(row.row_template_config_id, pool).await?;

    let slots = EditionSlot::find_by_row_with_content(row.id, pool).await?;

    Ok(EditionRowResult {
        id: row.id,
        row_template_slug: template
            .as_ref()
            .map(|t| t.slug.clone())
            .unwrap_or_default(),
        layout_variant: template
            .as_ref()
            .map(|t| t.layout_variant.clone())
            .unwrap_or_else(|| "full".to_string()),
        row_template_id: row.row_template_config_id,
        row_template_display_name: template
            .as_ref()
            .map(|t| t.display_name.clone())
            .unwrap_or_default(),
        row_template_description: template.and_then(|t| t.description),
        row_template_slots: template_slots
            .iter()
            .map(|s| RowTemplateSlotResult {
                slot_index: s.slot_index,
                weight: s.weight.clone(),
                count: s.count,
                accepts: s.accepts.clone(),
                post_template_slug: s.post_template_slug.clone(),
            })
            .collect(),
        sort_order: row.sort_order,
        section_id: row.section_id,
        slots: slots
            .iter()
            .map(|s| EditionSlotResult {
                id: s.id,
                kind: s.kind.clone(),
                slot_index: s.slot_index,
                post_id: s.post_id,
                post_template: s.post_template.clone(),
                post_title: s.post_title.clone(),
                post_post_type: s.post_post_type.clone(),
                post_weight: s.post_weight.clone(),
                post_status: s.post_status.clone(),
                widget_id: s.widget_id,
                widget_type: s.widget_type.clone(),
                widget_authoring_mode: s.widget_authoring_mode.clone(),
                widget_data: s.widget_data.clone(),
            })
            .collect(),
    })
}

/// Re-fetch a slot with embedded content data (post or widget).
async fn slot_with_content_data(
    slot: &EditionSlot,
    pool: &sqlx::PgPool,
) -> ApiResult<EditionSlotResult> {
    let slots_with_content =
        EditionSlot::find_by_row_with_content(slot.edition_row_id, pool).await?;

    match slots_with_content.into_iter().find(|s| s.id == slot.id) {
        Some(s) => Ok(EditionSlotResult {
            id: s.id,
            kind: s.kind,
            slot_index: s.slot_index,
            post_id: s.post_id,
            post_template: s.post_template,
            post_title: s.post_title,
            post_post_type: s.post_post_type,
            post_weight: s.post_weight,
            post_status: s.post_status,
            widget_id: s.widget_id,
            widget_type: s.widget_type,
            widget_authoring_mode: s.widget_authoring_mode,
            widget_data: s.widget_data,
        }),
        None => Ok(EditionSlotResult {
            id: slot.id,
            kind: slot.kind.clone(),
            slot_index: slot.slot_index,
            post_id: slot.post_id,
            post_template: slot.post_template.clone(),
            post_title: None,
            post_post_type: None,
            post_weight: None,
            post_status: None,
            widget_id: slot.widget_id,
            widget_type: None,
            widget_authoring_mode: None,
            widget_data: None,
        }),
    }
}

fn parse_date(s: &str, field: &str) -> ApiResult<NaiveDate> {
    s.parse::<NaiveDate>()
        .map_err(|e| ApiError::BadRequest(format!("Invalid {}: {}", field, e)))
}

// =============================================================================
// Handlers
// =============================================================================

async fn list_counties(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<CountyListResult>> {
    let counties = County::find_all(&state.deps.db_pool).await?;

    Ok(Json(CountyListResult {
        counties: counties
            .iter()
            .map(|c| CountyResult {
                id: c.id,
                fips_code: c.fips_code.clone(),
                name: c.name.clone(),
                state: c.state.clone(),
            })
            .collect(),
    }))
}

async fn get_county(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetCountyRequest>,
) -> ApiResult<Json<CountyResult>> {
    let county = County::find_by_id(req.id, &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("County not found: {}", req.id)))?;

    Ok(Json(CountyResult {
        id: county.id,
        fips_code: county.fips_code,
        name: county.name,
        state: county.state,
    }))
}

async fn list_editions(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListEditionsRequest>,
) -> ApiResult<Json<EditionListResult>> {
    let period_start = req
        .period_start
        .as_deref()
        .map(|s| parse_date(s, "period_start"))
        .transpose()?;
    let period_end = req
        .period_end
        .as_deref()
        .map(|s| parse_date(s, "period_end"))
        .transpose()?;

    let filters = EditionFilters {
        county_id: req.county_id,
        status: req.status,
        period_start,
        period_end,
        limit: req.limit.map(|l| l as i64),
        offset: req.offset.map(|o| o as i64),
    };

    let (editions, total_count) = Edition::list(&filters, &state.deps.db_pool).await?;

    Ok(Json(EditionListResult {
        editions: editions.iter().map(edition_to_result).collect(),
        total_count,
    }))
}

async fn latest_editions(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<EditionListResult>> {
    let editions = Edition::latest_per_county(&state.deps.db_pool).await?;
    let total_count = editions.len() as i64;

    // Batch-load row counts for all editions (single query)
    let edition_ids: Vec<Uuid> = editions.iter().map(|e| e.id).collect();
    let row_counts = EditionRow::count_by_edition_ids(&edition_ids, &state.deps.db_pool).await?;
    let count_map: HashMap<Uuid, i64> = row_counts.into_iter().collect();

    let results = editions
        .iter()
        .map(|e| {
            let mut result = edition_to_result(e);
            result.row_count = Some(*count_map.get(&e.id).unwrap_or(&0));
            result
        })
        .collect();

    Ok(Json(EditionListResult {
        editions: results,
        total_count,
    }))
}

async fn get_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetEditionRequest>,
) -> ApiResult<Json<EditionDetailResult>> {
    let edition = Edition::find_by_id(req.id, &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Edition not found: {}", req.id)))?;

    let detail = load_edition_detail(&edition, &state.deps.db_pool).await?;
    Ok(Json(detail))
}

async fn current_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<CurrentEditionRequest>,
) -> ApiResult<Json<EditionDetailResult>> {
    let edition = Edition::find_published(req.county_id, &state.deps.db_pool)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "No published edition for county: {}",
                req.county_id
            ))
        })?;

    let detail = load_edition_detail(&edition, &state.deps.db_pool).await?;
    Ok(Json(detail))
}

async fn create_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<CreateEditionRequest>,
) -> ApiResult<Json<EditionResult>> {
    let period_start = parse_date(&req.period_start, "period_start")?;
    let period_end = parse_date(&req.period_end, "period_end")?;

    let edition = activities::create_edition(
        req.county_id,
        period_start,
        period_end,
        req.title.as_deref(),
        &state.deps,
    )
    .await?;

    Ok(Json(edition_to_result(&edition)))
}

async fn generate_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GenerateEditionRequest>,
) -> ApiResult<Json<EditionResult>> {
    let edition = activities::generate_edition(req.id, &state.deps).await?;
    Ok(Json(edition_to_result(&edition)))
}

async fn publish_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<PublishEditionRequest>,
) -> ApiResult<Json<EditionResult>> {
    let edition = activities::publish_edition(req.id, &state.deps).await?;
    Ok(Json(edition_to_result(&edition)))
}

async fn archive_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ArchiveEditionRequest>,
) -> ApiResult<Json<EditionResult>> {
    let edition = activities::archive_edition(req.id, &state.deps).await?;
    Ok(Json(edition_to_result(&edition)))
}

async fn batch_generate(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<BatchGenerateRequest>,
) -> ApiResult<Json<BatchGenerateEditionsResult>> {
    let period_start = parse_date(&req.period_start, "period_start")?;
    let period_end = parse_date(&req.period_end, "period_end")?;

    let result =
        activities::batch_generate_editions(period_start, period_end, &state.deps).await?;

    Ok(Json(BatchGenerateEditionsResult {
        created: result.created,
        regenerated: result.regenerated,
        skipped: result.skipped,
        failed: result.failed,
        total_counties: result.total_counties,
    }))
}

async fn row_templates(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<RowTemplateListResult>> {
    let pool = &state.deps.db_pool;
    let configs = RowTemplateConfig::find_all(pool).await?;
    let all_slots = RowTemplateSlot::find_all(pool).await?;

    let templates = configs
        .into_iter()
        .map(|c| {
            let slots: Vec<RowTemplateSlotResult> = all_slots
                .iter()
                .filter(|s| s.row_template_config_id == c.id)
                .map(|s| RowTemplateSlotResult {
                    slot_index: s.slot_index,
                    weight: s.weight.clone(),
                    count: s.count,
                    accepts: s.accepts.clone(),
                    post_template_slug: s.post_template_slug.clone(),
                })
                .collect();
            RowTemplateResult {
                id: c.id,
                slug: c.slug,
                display_name: c.display_name,
                description: c.description,
                layout_variant: c.layout_variant,
                slots,
            }
        })
        .collect();

    Ok(Json(RowTemplateListResult { templates }))
}

async fn post_templates(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<PostTemplateListResult>> {
    let configs = PostTemplateConfig::find_all(&state.deps.db_pool).await?;

    Ok(Json(PostTemplateListResult {
        templates: configs
            .into_iter()
            .map(|c| PostTemplateResult {
                id: c.id,
                slug: c.slug,
                display_name: c.display_name,
                compatible_types: c.compatible_types,
                body_target: c.body_target,
                body_max: c.body_max,
                title_max: c.title_max,
                weight: c.weight,
            })
            .collect(),
    }))
}

async fn update_edition_row(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateEditionRowRequest>,
) -> ApiResult<Json<EditionRowResult>> {
    let pool = &state.deps.db_pool;

    // Resolve template slug to ID if provided
    let template_id = match &req.row_template_slug {
        Some(slug) => {
            let tmpl = RowTemplateConfig::find_by_slug(slug, pool)
                .await?
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Row template not found: {}", slug))
                })?;
            Some(tmpl.id)
        }
        None => None,
    };

    let row = EditionRow::update(req.row_id, template_id, req.sort_order, pool).await?;
    let result = build_row_result(&row, pool).await?;
    Ok(Json(result))
}

async fn reorder_rows(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ReorderRowsRequest>,
) -> ApiResult<Json<ReorderRowsResult>> {
    let pool = &state.deps.db_pool;

    let rows = EditionRow::reorder(req.edition_id, &req.row_ids, pool).await?;

    // Load all templates + slots upfront (avoids N+1)
    let all_templates = RowTemplateConfig::find_all(pool).await?;
    let all_template_slots = RowTemplateSlot::find_all(pool).await?;

    let mut results = Vec::new();
    for row in &rows {
        let template = all_templates
            .iter()
            .find(|t| t.id == row.row_template_config_id);
        let template_slot_results: Vec<RowTemplateSlotResult> = all_template_slots
            .iter()
            .filter(|s| s.row_template_config_id == row.row_template_config_id)
            .map(|s| RowTemplateSlotResult {
                slot_index: s.slot_index,
                weight: s.weight.clone(),
                count: s.count,
                accepts: s.accepts.clone(),
                post_template_slug: s.post_template_slug.clone(),
            })
            .collect();

        let slots = EditionSlot::find_by_row_with_content(row.id, pool).await?;

        results.push(EditionRowResult {
            id: row.id,
            row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
            layout_variant: template.map(|t| t.layout_variant.clone()).unwrap_or_else(|| "full".to_string()),
            row_template_id: row.row_template_config_id,
            row_template_display_name: template
                .map(|t| t.display_name.clone())
                .unwrap_or_default(),
            row_template_description: template.and_then(|t| t.description.clone()),
            row_template_slots: template_slot_results,
            sort_order: row.sort_order,
            section_id: row.section_id,
            slots: slots
                .iter()
                .map(|s| EditionSlotResult {
                    id: s.id,
                    kind: s.kind.clone(),
                    slot_index: s.slot_index,
                    post_id: s.post_id,
                    post_template: s.post_template.clone(),
                    post_title: s.post_title.clone(),
                    post_post_type: s.post_post_type.clone(),
                    post_weight: s.post_weight.clone(),
                    post_status: s.post_status.clone(),
                    widget_id: s.widget_id,
                    widget_type: s.widget_type.clone(),
                    widget_authoring_mode: s.widget_authoring_mode.clone(),
                    widget_data: s.widget_data.clone(),
                })
                .collect(),
        });
    }

    Ok(Json(ReorderRowsResult { rows: results }))
}

async fn remove_post(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<RemovePostFromEditionRequest>,
) -> ApiResult<Json<bool>> {
    EditionSlot::delete(req.slot_id, &state.deps.db_pool).await?;
    Ok(Json(true))
}

async fn change_slot_template(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ChangeSlotTemplateRequest>,
) -> ApiResult<Json<EditionSlotResult>> {
    let pool = &state.deps.db_pool;
    let slot =
        EditionSlot::change_template(req.slot_id, &req.post_template, pool).await?;

    let result = slot_with_content_data(&slot, pool).await?;
    Ok(Json(result))
}

async fn move_slot(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<MoveSlotRequest>,
) -> ApiResult<Json<EditionSlotResult>> {
    let pool = &state.deps.db_pool;
    let slot =
        EditionSlot::move_to(req.slot_id, req.target_row_id, req.slot_index, pool)
            .await?;

    let result = slot_with_content_data(&slot, pool).await?;
    Ok(Json(result))
}

async fn add_post_to_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<AddPostToEditionRequest>,
) -> ApiResult<Json<EditionSlotResult>> {
    let pool = &state.deps.db_pool;
    let slot = EditionSlot::create(
        req.edition_row_id,
        req.post_id,
        &req.post_template,
        req.slot_index,
        pool,
    )
    .await?;

    let result = slot_with_content_data(&slot, pool).await?;
    Ok(Json(result))
}

async fn add_edition_row(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<AddEditionRowRequest>,
) -> ApiResult<Json<EditionRowResult>> {
    let pool = &state.deps.db_pool;

    // Resolve template slug to ID
    let template = RowTemplateConfig::find_by_slug(&req.row_template_slug, pool)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Row template not found: {}",
                req.row_template_slug
            ))
        })?;

    let row =
        EditionRow::create(req.edition_id, template.id, req.sort_order, pool).await?;

    let template_slots =
        RowTemplateSlot::find_by_template(template.id, pool).await?;

    Ok(Json(EditionRowResult {
        id: row.id,
        row_template_slug: template.slug,
        layout_variant: template.layout_variant,
        row_template_id: template.id,
        row_template_display_name: template.display_name,
        row_template_description: template.description,
        row_template_slots: template_slots
            .iter()
            .map(|s| RowTemplateSlotResult {
                slot_index: s.slot_index,
                weight: s.weight.clone(),
                count: s.count,
                accepts: s.accepts.clone(),
                post_template_slug: s.post_template_slug.clone(),
            })
            .collect(),
        sort_order: row.sort_order,
        section_id: row.section_id,
        slots: vec![],
    }))
}

async fn delete_edition_row(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteEditionRowRequest>,
) -> ApiResult<Json<bool>> {
    EditionRow::delete(req.row_id, &state.deps.db_pool).await?;
    Ok(Json(true))
}

async fn review_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ReviewEditionRequest>,
) -> ApiResult<Json<EditionResult>> {
    let edition = activities::review_edition(req.id, &state.deps).await?;
    Ok(Json(edition_to_result(&edition)))
}

async fn approve_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ApproveEditionRequest>,
) -> ApiResult<Json<EditionResult>> {
    let edition = activities::approve_edition(req.id, &state.deps).await?;
    Ok(Json(edition_to_result(&edition)))
}

async fn batch_approve_editions(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<BatchApproveEditionsRequest>,
) -> ApiResult<Json<BatchEditionsResult>> {
    let (succeeded, failed) =
        activities::batch_approve_editions(&req.ids, &state.deps).await?;
    Ok(Json(BatchEditionsResult { succeeded, failed }))
}

async fn batch_publish_editions(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<BatchPublishEditionsRequest>,
) -> ApiResult<Json<BatchEditionsResult>> {
    let (succeeded, failed) =
        activities::batch_publish_editions(&req.ids, &state.deps).await?;
    Ok(Json(BatchEditionsResult { succeeded, failed }))
}

async fn edition_kanban_stats(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<EditionKanbanStatsRequest>,
) -> ApiResult<Json<EditionKanbanStatsResult>> {
    let period_start = parse_date(&req.period_start, "period_start")?;
    let period_end = parse_date(&req.period_end, "period_end")?;

    let counts =
        Edition::count_by_status(period_start, period_end, &state.deps.db_pool).await?;

    let mut result = EditionKanbanStatsResult {
        draft: 0,
        in_review: 0,
        approved: 0,
        published: 0,
    };

    for (status, count) in counts {
        match status.as_str() {
            "draft" => result.draft = count as i32,
            "in_review" => result.in_review = count as i32,
            "approved" => result.approved = count as i32,
            "published" => result.published = count as i32,
            _ => {} // archived or other statuses ignored for kanban
        }
    }

    Ok(Json(result))
}

async fn add_widget_to_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<AddWidgetToEditionRequest>,
) -> ApiResult<Json<EditionSlotResult>> {
    let pool = &state.deps.db_pool;

    // Verify widget exists
    Widget::find_by_id(req.widget_id, pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Widget not found: {}", req.widget_id)))?;

    let slot = EditionSlot::create_widget_slot(
        req.edition_row_id,
        req.widget_id,
        req.slot_index,
        pool,
    )
    .await?;

    let result = slot_with_content_data(&slot, pool).await?;
    Ok(Json(result))
}

// =============================================================================
// Public broadsheet handler (no auth required)
// =============================================================================

async fn public_current_broadsheet(
    State(state): State<AppState>,
    // No AdminUser — public endpoint
    Json(req): Json<CurrentEditionRequest>,
) -> ApiResult<Json<PublicBroadsheetResult>> {
    let pool = &state.deps.db_pool;

    let edition = Edition::find_published(req.county_id, pool)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "No published edition for county: {}",
                req.county_id
            ))
        })?;

    let county = County::find_by_id(edition.county_id, pool)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("County not found: {}", edition.county_id))
        })?;

    let broadsheet_result = build_public_broadsheet(&edition, &county, &state).await?;
    Ok(Json(broadsheet_result))
}

// =============================================================================
// Preview broadsheet (admin auth required, any edition status)
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct PreviewBroadsheetRequest {
    pub edition_id: Uuid,
}

async fn preview_broadsheet(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<PreviewBroadsheetRequest>,
) -> ApiResult<Json<PublicBroadsheetResult>> {
    let pool = &state.deps.db_pool;

    let edition = Edition::find_by_id(req.edition_id, pool)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Edition not found: {}", req.edition_id))
        })?;

    let county = County::find_by_id(edition.county_id, pool)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("County not found: {}", edition.county_id))
        })?;

    let broadsheet_result = build_public_broadsheet(&edition, &county, &state).await?;
    Ok(Json(broadsheet_result))
}

/// Shared logic for building a public broadsheet response (used by both public and preview).
async fn build_public_broadsheet(
    edition: &Edition,
    county: &County,
    state: &AppState,
) -> ApiResult<PublicBroadsheetResult> {
    let pool = &state.deps.db_pool;
    let rows = EditionRow::find_by_edition(edition.id, pool).await?;
    let all_templates = RowTemplateConfig::find_all(pool).await?;

    // Collect all post IDs across all rows for batch loading
    let mut all_slots_by_row: Vec<Vec<EditionSlot>> = Vec::new();
    let mut all_post_ids: Vec<Uuid> = Vec::new();
    let mut all_widget_ids: Vec<Uuid> = Vec::new();

    for row in &rows {
        let slots = EditionSlot::find_by_row(row.id, pool).await?;
        for slot in &slots {
            if let Some(post_id) = slot.post_id {
                all_post_ids.push(post_id);
            }
            if let Some(widget_id) = slot.widget_id {
                all_widget_ids.push(widget_id);
            }
        }
        all_slots_by_row.push(slots);
    }

    // Batch load full post data, tags, urgent notes, and org info
    let posts_by_id: HashMap<Uuid, Post> = if !all_post_ids.is_empty() {
        Post::find_by_ids(&all_post_ids, pool)
            .await?
            .into_iter()
            .map(|p| (p.id.into_uuid(), p))
            .collect()
    } else {
        HashMap::new()
    };

    // Batch load widgets
    let widgets_by_id: HashMap<Uuid, Widget> = if !all_widget_ids.is_empty() {
        let mut map = HashMap::new();
        for wid in &all_widget_ids {
            if let Some(w) = Widget::find_by_id(*wid, pool).await? {
                map.insert(w.id, w);
            }
        }
        map
    } else {
        HashMap::new()
    };

    let (mut tags_by_post, mut urgent_notes_by_post) =
        load_tags_and_notes(&all_post_ids, &state.deps).await?;

    let mut org_info = Post::find_org_info_for_posts(&all_post_ids, pool).await?;

    // Batch load contacts for all posts
    let all_contacts = Contact::find_by_post_ids(&all_post_ids, pool).await?;
    let mut contacts_by_post: HashMap<Uuid, Vec<BroadsheetContactResult>> = HashMap::new();
    for c in all_contacts {
        contacts_by_post
            .entry(c.contactable_id)
            .or_default()
            .push(BroadsheetContactResult {
                contact_type: c.contact_type,
                contact_value: c.contact_value,
                contact_label: c.contact_label,
            });
    }

    // Batch load field groups for all posts
    let all_media = PostMediaRecord::find_by_post_ids(&all_post_ids, pool).await?;
    let mut media_by_post: HashMap<Uuid, Vec<BroadsheetMediaResult>> = HashMap::new();
    for m in all_media {
        media_by_post
            .entry(m.post_id)
            .or_default()
            .push(BroadsheetMediaResult {
                image_url: m.image_url,
                caption: m.caption,
                credit: m.credit,
            });
    }

    let all_items = PostItem::find_by_post_ids(&all_post_ids, pool).await?;
    let mut items_by_post: HashMap<Uuid, Vec<BroadsheetItemResult>> = HashMap::new();
    for item in all_items {
        items_by_post
            .entry(item.post_id)
            .or_default()
            .push(BroadsheetItemResult {
                name: item.name,
                detail: item.detail,
            });
    }

    let all_schedule = PostScheduleEntry::find_by_post_ids(&all_post_ids, pool).await?;
    let mut schedule_by_post: HashMap<Uuid, Vec<BroadsheetScheduleEntryResult>> = HashMap::new();
    for entry in all_schedule {
        schedule_by_post
            .entry(entry.post_id)
            .or_default()
            .push(BroadsheetScheduleEntryResult {
                day: entry.day,
                opens: entry.opens,
                closes: entry.closes,
            });
    }

    // 1:1 field groups
    let all_persons = PostPersonRecord::find_by_post_ids(&all_post_ids, pool).await?;
    let mut persons_by_post: HashMap<Uuid, BroadsheetPersonResult> = all_persons
        .into_iter()
        .map(|p| (p.post_id, BroadsheetPersonResult {
            name: p.name,
            role: p.role,
            bio: p.bio,
            photo_url: p.photo_url,
            quote: p.quote,
        }))
        .collect();

    let all_links = PostLinkRecord::find_by_post_ids(&all_post_ids, pool).await?;
    let mut links_by_post: HashMap<Uuid, BroadsheetLinkResult> = all_links
        .into_iter()
        .map(|l| (l.post_id, BroadsheetLinkResult {
            label: l.label,
            url: l.url,
            deadline: l.deadline.map(|d| d.to_string()),
        }))
        .collect();

    let all_source_attrs = PostSourceAttr::find_by_post_ids(&all_post_ids, pool).await?;
    let mut source_attrs_by_post: HashMap<Uuid, BroadsheetSourceAttributionResult> = all_source_attrs
        .into_iter()
        .map(|s| (s.post_id, BroadsheetSourceAttributionResult {
            source_name: s.source_name,
            attribution: s.attribution,
        }))
        .collect();

    let all_metas = PostMetaRecord::find_by_post_ids(&all_post_ids, pool).await?;
    let mut metas_by_post: HashMap<Uuid, BroadsheetMetaResult> = all_metas
        .into_iter()
        .map(|m| (m.post_id, BroadsheetMetaResult {
            kicker: m.kicker,
            byline: m.byline,
            timestamp: m.timestamp.map(|t| t.to_rfc3339()),
            updated: m.updated,
            deck: m.deck,
        }))
        .collect();

    let all_datetimes = PostDatetimeRecord::find_by_post_ids(&all_post_ids, pool).await?;
    let mut datetimes_by_post: HashMap<Uuid, BroadsheetDatetimeResult> = all_datetimes
        .into_iter()
        .map(|d| (d.post_id, BroadsheetDatetimeResult {
            start: d.start_at.map(|t| t.to_rfc3339()),
            end: d.end_at.map(|t| t.to_rfc3339()),
            cost: d.cost,
            recurring: d.recurring,
        }))
        .collect();

    let all_statuses = PostStatusRecord::find_by_post_ids(&all_post_ids, pool).await?;
    let mut statuses_by_post: HashMap<Uuid, BroadsheetStatusResult> = all_statuses
        .into_iter()
        .map(|s| (s.post_id, BroadsheetStatusResult {
            state: s.state,
            verified: s.verified,
        }))
        .collect();

    // Assemble rows
    let mut row_results = Vec::new();
    for (row, slots) in rows.iter().zip(all_slots_by_row.iter()) {
        let template = all_templates
            .iter()
            .find(|t| t.id == row.row_template_config_id);

        let slot_results: Vec<PublicBroadsheetSlotResult> = slots
            .iter()
            .filter_map(|slot| {
                match slot.kind.as_str() {
                    "post" => {
                        let post_id = slot.post_id?;
                        let post = posts_by_id.get(&post_id)?;
                        let id = post.id.into_uuid();
                        let org_name = org_info.remove(&id).map(|(_, name)| name);

                        Some(PublicBroadsheetSlotResult {
                            kind: "post".to_string(),
                            post_template: slot.post_template.clone(),
                            widget_template: None,
                            slot_index: slot.slot_index,
                            post: Some(PublicBroadsheetPostResult {
                                id,
                                title: post.title.clone(),
                                body_raw: post.body_raw.clone(),
                                post_type: post.post_type.clone(),
                                weight: post.weight.clone(),
                                is_urgent: post.is_urgent,
                                location: post.location.clone(),
                                organization_name: org_name,
                                published_at: post.published_at.map(|dt| dt.to_rfc3339()),
                                tags: tags_by_post.remove(&id).unwrap_or_default(),
                                contacts: contacts_by_post.remove(&id).unwrap_or_default(),
                                urgent_notes: urgent_notes_by_post.remove(&id).unwrap_or_default(),
                                body_heavy: post.body_heavy.clone(),
                                body_medium: post.body_medium.clone(),
                                body_light: post.body_light.clone(),
                                // Field groups
                                media: media_by_post.remove(&id).unwrap_or_default(),
                                items: items_by_post.remove(&id).unwrap_or_default(),
                                person: persons_by_post.remove(&id),
                                link: links_by_post.remove(&id),
                                source_attribution: source_attrs_by_post.remove(&id),
                                meta: metas_by_post.remove(&id),
                                datetime: datetimes_by_post.remove(&id),
                                post_status: statuses_by_post.remove(&id),
                                schedule: schedule_by_post.remove(&id).unwrap_or_default(),
                            }),
                            widget: None,
                        })
                    }
                    "widget" => {
                        let widget_id = slot.widget_id?;
                        let widget = widgets_by_id.get(&widget_id)?;

                        Some(PublicBroadsheetSlotResult {
                            kind: "widget".to_string(),
                            post_template: None,
                            widget_template: slot.widget_template.clone(),
                            slot_index: slot.slot_index,
                            post: None,
                            widget: Some(PublicBroadsheetWidgetResult {
                                id: widget.id,
                                widget_type: widget.widget_type.clone(),
                                authoring_mode: widget.authoring_mode.clone(),
                                data: widget.data.clone(),
                            }),
                        })
                    }
                    _ => None,
                }
            })
            .collect();

        row_results.push(PublicBroadsheetRowResult {
            row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
            layout_variant: template.map(|t| t.layout_variant.clone()).unwrap_or_else(|| "full".to_string()),
            sort_order: row.sort_order,
            section_id: row.section_id,
            slots: slot_results,
        });
    }

    // Load sections
    let sections = EditionSection::find_by_edition(edition.id, pool).await?;
    let section_results: Vec<EditionSectionResult> = sections
        .iter()
        .map(|s| section_to_result(s))
        .collect();

    Ok(PublicBroadsheetResult {
        edition: edition_to_result(edition),
        county: CountyResult {
            id: county.id,
            fips_code: county.fips_code.clone(),
            name: county.name.clone(),
            state: county.state.clone(),
        },
        rows: row_results,
        sections: section_results,
    })
}

// =============================================================================
// Section CRUD handlers
// =============================================================================

async fn add_section(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<AddSectionRequest>,
) -> ApiResult<Json<EditionSectionResult>> {
    let pool = &state.deps.db_pool;
    let section = EditionSection::create(
        req.edition_id,
        &req.title,
        req.subtitle.as_deref(),
        req.topic_slug.as_deref(),
        req.sort_order,
        pool,
    )
    .await?;
    Ok(Json(section_to_result(&section)))
}

async fn update_section(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateSectionRequest>,
) -> ApiResult<Json<EditionSectionResult>> {
    let pool = &state.deps.db_pool;
    let section = EditionSection::update(
        req.id,
        req.title.as_deref(),
        req.subtitle.as_ref().map(|s| s.as_deref()),
        req.topic_slug.as_ref().map(|s| s.as_deref()),
        pool,
    )
    .await?;
    Ok(Json(section_to_result(&section)))
}

async fn reorder_sections(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ReorderSectionsRequest>,
) -> ApiResult<Json<Vec<EditionSectionResult>>> {
    let pool = &state.deps.db_pool;
    let sections = EditionSection::reorder(req.edition_id, &req.section_ids, pool).await?;
    Ok(Json(sections.iter().map(section_to_result).collect()))
}

async fn delete_section(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteSectionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = &state.deps.db_pool;
    EditionSection::delete(req.id, pool).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn assign_row_to_section(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<AssignRowToSectionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = &state.deps.db_pool;
    EditionRow::assign_to_section(req.row_id, req.section_id, pool).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // Public (no auth)
        .route("/Public/current_broadsheet", post(public_current_broadsheet))
        // Admin
        .route("/Editions/list_counties", post(list_counties))
        .route("/Editions/get_county", post(get_county))
        .route("/Editions/list_editions", post(list_editions))
        .route("/Editions/latest_editions", post(latest_editions))
        .route("/Editions/get_edition", post(get_edition))
        .route("/Editions/current_edition", post(current_edition))
        .route("/Editions/create_edition", post(create_edition))
        .route("/Editions/generate_edition", post(generate_edition))
        .route("/Editions/publish_edition", post(publish_edition))
        .route("/Editions/archive_edition", post(archive_edition))
        .route("/Editions/batch_generate", post(batch_generate))
        .route("/Editions/row_templates", post(row_templates))
        .route("/Editions/post_templates", post(post_templates))
        .route("/Editions/update_edition_row", post(update_edition_row))
        .route("/Editions/reorder_rows", post(reorder_rows))
        .route("/Editions/remove_post", post(remove_post))
        .route("/Editions/change_slot_template", post(change_slot_template))
        .route("/Editions/move_slot", post(move_slot))
        .route("/Editions/add_post_to_edition", post(add_post_to_edition))
        .route("/Editions/add_edition_row", post(add_edition_row))
        .route("/Editions/delete_edition_row", post(delete_edition_row))
        .route("/Editions/review_edition", post(review_edition))
        .route("/Editions/approve_edition", post(approve_edition))
        .route(
            "/Editions/batch_approve_editions",
            post(batch_approve_editions),
        )
        .route(
            "/Editions/batch_publish_editions",
            post(batch_publish_editions),
        )
        .route(
            "/Editions/edition_kanban_stats",
            post(edition_kanban_stats),
        )
        .route("/Editions/add_widget_to_edition", post(add_widget_to_edition))
        // Preview
        .route("/Editions/preview_broadsheet", post(preview_broadsheet))
        // Section CRUD
        .route("/Editions/add_section", post(add_section))
        .route("/Editions/update_section", post(update_section))
        .route("/Editions/reorder_sections", post(reorder_sections))
        .route("/Editions/delete_section", post(delete_section))
        .route("/Editions/assign_row_to_section", post(assign_row_to_section))
}
