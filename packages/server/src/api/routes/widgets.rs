use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::domains::editions::Edition;
use crate::domains::widgets::models::widget::{CreateWidgetParams, UpdateWidgetParams, WidgetFilters};
use crate::domains::widgets::Widget;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateWidgetRequest {
    pub widget_type: String,
    pub authoring_mode: Option<String>,
    pub data: serde_json::Value,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub county_id: Option<Uuid>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetWidgetRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListWidgetsRequest {
    pub widget_type: Option<String>,
    pub county_id: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWidgetRequest {
    pub id: Uuid,
    pub data: Option<serde_json::Value>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub county_id: Option<Uuid>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteWidgetRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListWidgetsForEditionRequest {
    pub edition_id: Uuid,
    pub slotted_filter: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct WidgetResult {
    pub id: Uuid,
    pub widget_type: String,
    pub authoring_mode: String,
    pub data: serde_json::Value,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub county_id: Option<Uuid>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct WidgetListResult {
    pub widgets: Vec<WidgetResult>,
}

// =============================================================================
// Helpers
// =============================================================================

fn widget_to_result(w: &Widget) -> WidgetResult {
    WidgetResult {
        id: w.id,
        widget_type: w.widget_type.clone(),
        authoring_mode: w.authoring_mode.clone(),
        data: w.data.clone(),
        zip_code: w.zip_code.clone(),
        city: w.city.clone(),
        county_id: w.county_id,
        start_date: w.start_date.map(|d| d.to_string()),
        end_date: w.end_date.map(|d| d.to_string()),
        created_at: w.created_at.to_rfc3339(),
        updated_at: w.updated_at.to_rfc3339(),
    }
}

fn parse_date(s: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| ApiError::BadRequest(format!("Invalid date '{}': {}", s, e)))
}

// =============================================================================
// Handlers
// =============================================================================

async fn create_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<CreateWidgetRequest>,
) -> ApiResult<Json<WidgetResult>> {
    let authoring_mode = req.authoring_mode.as_deref().unwrap_or("human");

    let params = CreateWidgetParams {
        zip_code: req.zip_code,
        city: req.city,
        county_id: req.county_id,
        start_date: req.start_date.as_deref().map(parse_date).transpose()?,
        end_date: req.end_date.as_deref().map(parse_date).transpose()?,
    };

    let widget = Widget::create(
        &req.widget_type,
        authoring_mode,
        req.data,
        params,
        &state.deps.db_pool,
    )
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(widget_to_result(&widget)))
}

async fn get_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetWidgetRequest>,
) -> ApiResult<Json<WidgetResult>> {
    let widget = Widget::find_by_id(req.id, &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Widget not found: {}", req.id)))?;

    Ok(Json(widget_to_result(&widget)))
}

async fn list_widgets(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListWidgetsRequest>,
) -> ApiResult<Json<WidgetListResult>> {
    let limit = req.limit.unwrap_or(50);
    let offset = req.offset.unwrap_or(0);

    let filters = WidgetFilters {
        widget_type: req.widget_type.as_deref(),
        county_id: req.county_id,
        search: req.search.as_deref(),
    };

    let widgets = Widget::find_all(&filters, limit, offset, &state.deps.db_pool).await?;

    Ok(Json(WidgetListResult {
        widgets: widgets.iter().map(widget_to_result).collect(),
    }))
}

async fn update_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateWidgetRequest>,
) -> ApiResult<Json<WidgetResult>> {
    // Map request fields to UpdateWidgetParams.
    // Some("") means "clear", Some("value") means "set", None means "don't change".
    let params = UpdateWidgetParams {
        data: req.data,
        zip_code: req.zip_code.map(|s| if s.is_empty() { None } else { Some(s) }),
        city: req.city.map(|s| if s.is_empty() { None } else { Some(s) }),
        county_id: req.county_id.map(Some),
        start_date: match req.start_date {
            Some(ref s) => Some(Some(parse_date(s)?)),
            None => None,
        },
        end_date: match req.end_date {
            Some(ref s) => Some(Some(parse_date(s)?)),
            None => None,
        },
    };

    let widget = Widget::update(req.id, params, &state.deps.db_pool)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(widget_to_result(&widget)))
}

async fn delete_widget(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteWidgetRequest>,
) -> ApiResult<Json<bool>> {
    Widget::delete(req.id, &state.deps.db_pool).await?;
    Ok(Json(true))
}

async fn list_widgets_for_edition(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListWidgetsForEditionRequest>,
) -> ApiResult<Json<WidgetListResult>> {
    let edition = Edition::find_by_id(req.edition_id, &state.deps.db_pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Edition not found: {}", req.edition_id)))?;

    let slotted_filter = req.slotted_filter.as_deref().unwrap_or("all");
    let limit = req.limit.unwrap_or(50);
    let offset = req.offset.unwrap_or(0);

    let today = chrono::Utc::now().date_naive();

    let widgets = Widget::find_for_edition(
        edition.county_id,
        today,
        edition.id,
        slotted_filter,
        limit,
        offset,
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(WidgetListResult {
        widgets: widgets.iter().map(widget_to_result).collect(),
    }))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Widgets/create_widget", post(create_widget))
        .route("/Widgets/get_widget", post(get_widget))
        .route("/Widgets/list_widgets", post(list_widgets))
        .route("/Widgets/update_widget", post(update_widget))
        .route("/Widgets/delete_widget", post(delete_widget))
        .route(
            "/Widgets/list_widgets_for_edition",
            post(list_widgets_for_edition),
        )
}
