use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::domains::editions::activities;
use crate::domains::editions::models::county::County;
use crate::domains::editions::models::edition::{Edition, EditionFilters};
use crate::domains::editions::models::edition_row::EditionRow;
use crate::domains::editions::models::edition_slot::EditionSlot;
use crate::domains::editions::models::edition_widget::EditionWidget;
use crate::domains::editions::models::post_template_config::PostTemplateConfig;
use crate::domains::editions::models::row_template_config::RowTemplateConfig;
use crate::domains::editions::models::row_template_slot::RowTemplateSlot;

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
pub struct AddWidgetRequest {
    pub edition_row_id: Uuid,
    pub widget_type: String,
    pub slot_index: i32,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWidgetRequest {
    pub id: Uuid,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RemoveWidgetRequest {
    pub id: Uuid,
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
}

#[derive(Debug, Serialize)]
pub struct EditionRowResult {
    pub id: Uuid,
    pub row_template_slug: String,
    pub row_template_id: Uuid,
    pub row_template_display_name: String,
    pub row_template_description: Option<String>,
    pub row_template_slots: Vec<RowTemplateSlotResult>,
    pub sort_order: i32,
    pub slots: Vec<EditionSlotResult>,
    pub widgets: Vec<EditionWidgetResult>,
}

#[derive(Debug, Serialize)]
pub struct EditionWidgetResult {
    pub id: Uuid,
    pub widget_type: String,
    pub slot_index: i32,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct EditionSlotResult {
    pub id: Uuid,
    pub post_id: Uuid,
    pub post_template: String,
    pub slot_index: i32,
    pub post_title: Option<String>,
    pub post_post_type: Option<String>,
    pub post_weight: Option<String>,
    pub post_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RowTemplateResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub slots: Vec<RowTemplateSlotResult>,
}

#[derive(Debug, Serialize)]
pub struct RowTemplateSlotResult {
    pub slot_index: i32,
    pub weight: String,
    pub count: i32,
    pub accepts: Option<Vec<String>>,
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
}

#[derive(Debug, Serialize)]
pub struct PostTemplateListResult {
    pub templates: Vec<PostTemplateResult>,
}

#[derive(Debug, Serialize)]
pub struct BatchGenerateEditionsResult {
    pub created: i32,
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
            })
            .collect();

        let slots = EditionSlot::find_by_row_with_posts(row.id, pool).await?;

        let widgets = EditionWidget::find_by_row(row.id, pool).await?;

        row_results.push(EditionRowResult {
            id: row.id,
            row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
            row_template_id: row.row_template_config_id,
            row_template_display_name: template
                .map(|t| t.display_name.clone())
                .unwrap_or_default(),
            row_template_description: template.and_then(|t| t.description.clone()),
            row_template_slots: template_slot_results,
            sort_order: row.sort_order,
            slots: slots
                .iter()
                .map(|s| EditionSlotResult {
                    id: s.id,
                    post_id: s.post_id,
                    post_template: s.post_template.clone(),
                    slot_index: s.slot_index,
                    post_title: Some(s.post_title.clone()),
                    post_post_type: s.post_post_type.clone(),
                    post_weight: s.post_weight.clone(),
                    post_status: Some(s.post_status.clone()),
                })
                .collect(),
            widgets: widgets
                .iter()
                .map(|w| EditionWidgetResult {
                    id: w.id,
                    widget_type: w.widget_type.clone(),
                    slot_index: w.slot_index,
                    config: w.config.clone(),
                })
                .collect(),
        });
    }

    Ok(EditionDetailResult {
        edition: edition_to_result(edition),
        rows: row_results,
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

    let slots = EditionSlot::find_by_row_with_posts(row.id, pool).await?;

    Ok(EditionRowResult {
        id: row.id,
        row_template_slug: template
            .as_ref()
            .map(|t| t.slug.clone())
            .unwrap_or_default(),
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
            })
            .collect(),
        sort_order: row.sort_order,
        slots: slots
            .iter()
            .map(|s| EditionSlotResult {
                id: s.id,
                post_id: s.post_id,
                post_template: s.post_template.clone(),
                slot_index: s.slot_index,
                post_title: Some(s.post_title.clone()),
                post_post_type: s.post_post_type.clone(),
                post_weight: s.post_weight.clone(),
                post_status: Some(s.post_status.clone()),
            })
            .collect(),
        widgets: vec![],
    })
}

/// Re-fetch a slot with embedded post data.
async fn slot_with_post_data(
    slot: &EditionSlot,
    pool: &sqlx::PgPool,
) -> ApiResult<EditionSlotResult> {
    let slots_with_posts =
        EditionSlot::find_by_row_with_posts(slot.edition_row_id, pool).await?;

    match slots_with_posts.into_iter().find(|s| s.id == slot.id) {
        Some(s) => Ok(EditionSlotResult {
            id: s.id,
            post_id: s.post_id,
            post_template: s.post_template,
            slot_index: s.slot_index,
            post_title: Some(s.post_title),
            post_post_type: s.post_post_type,
            post_weight: s.post_weight,
            post_status: Some(s.post_status),
        }),
        None => Ok(EditionSlotResult {
            id: slot.id,
            post_id: slot.post_id,
            post_template: slot.post_template.clone(),
            slot_index: slot.slot_index,
            post_title: None,
            post_post_type: None,
            post_weight: None,
            post_status: None,
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

    Ok(Json(EditionListResult {
        editions: editions.iter().map(edition_to_result).collect(),
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
                })
                .collect();
            RowTemplateResult {
                id: c.id,
                slug: c.slug,
                display_name: c.display_name,
                description: c.description,
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
            })
            .collect();

        let slots = EditionSlot::find_by_row_with_posts(row.id, pool).await?;

        results.push(EditionRowResult {
            id: row.id,
            row_template_slug: template.map(|t| t.slug.clone()).unwrap_or_default(),
            row_template_id: row.row_template_config_id,
            row_template_display_name: template
                .map(|t| t.display_name.clone())
                .unwrap_or_default(),
            row_template_description: template.and_then(|t| t.description.clone()),
            row_template_slots: template_slot_results,
            sort_order: row.sort_order,
            slots: slots
                .iter()
                .map(|s| EditionSlotResult {
                    id: s.id,
                    post_id: s.post_id,
                    post_template: s.post_template.clone(),
                    slot_index: s.slot_index,
                    post_title: Some(s.post_title.clone()),
                    post_post_type: s.post_post_type.clone(),
                    post_weight: s.post_weight.clone(),
                    post_status: Some(s.post_status.clone()),
                })
                .collect(),
            widgets: vec![],
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

    let result = slot_with_post_data(&slot, pool).await?;
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

    let result = slot_with_post_data(&slot, pool).await?;
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

    let result = slot_with_post_data(&slot, pool).await?;
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
            })
            .collect(),
        sort_order: row.sort_order,
        slots: vec![],
        widgets: vec![],
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

async fn add_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<AddWidgetRequest>,
) -> ApiResult<Json<EditionWidgetResult>> {
    let widget = EditionWidget::create(
        req.edition_row_id,
        &req.widget_type,
        req.slot_index,
        req.config,
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(EditionWidgetResult {
        id: widget.id,
        widget_type: widget.widget_type,
        slot_index: widget.slot_index,
        config: widget.config,
    }))
}

async fn update_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateWidgetRequest>,
) -> ApiResult<Json<EditionWidgetResult>> {
    let widget =
        EditionWidget::update(req.id, req.config, &state.deps.db_pool).await?;

    Ok(Json(EditionWidgetResult {
        id: widget.id,
        widget_type: widget.widget_type,
        slot_index: widget.slot_index,
        config: widget.config,
    }))
}

async fn remove_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<RemoveWidgetRequest>,
) -> ApiResult<Json<bool>> {
    EditionWidget::delete(req.id, &state.deps.db_pool).await?;
    Ok(Json(true))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
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
        .route("/Editions/add_widget", post(add_widget))
        .route("/Editions/update_widget", post(update_widget))
        .route("/Editions/remove_widget", post(remove_widget))
}
