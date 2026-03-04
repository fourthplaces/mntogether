use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::common::PaginationArgs;
use crate::domains::member::activities;
use crate::domains::member::models::member::Member;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ListMembersRequest {
    pub first: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WeeklyResetRequest {}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct MemberResult {
    pub id: Uuid,
    pub searchable_text: String,
    pub location_name: Option<String>,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct MemberListResult {
    pub members: Vec<MemberResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

#[derive(Debug, Serialize)]
pub struct WeeklyResetResult {
    pub members_reset: i64,
}

// =============================================================================
// Handlers
// =============================================================================

async fn list(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListMembersRequest>,
) -> ApiResult<Json<MemberListResult>> {
    let pagination_args = PaginationArgs {
        first: req.first,
        after: req.after,
        last: req.last,
        before: req.before,
    };
    let validated = pagination_args
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let connection = activities::get_members_paginated(&validated, &state.deps).await?;

    Ok(Json(MemberListResult {
        members: connection
            .edges
            .into_iter()
            .filter_map(|e| {
                uuid::Uuid::parse_str(&e.node.id)
                    .ok()
                    .map(|id| MemberResult {
                        id,
                        searchable_text: e.node.searchable_text,
                        location_name: e.node.location_name,
                        active: e.node.active,
                        created_at: e.node.created_at.to_rfc3339(),
                    })
            })
            .collect(),
        total_count: connection.total_count,
        has_next_page: connection.page_info.has_next_page,
        has_previous_page: connection.page_info.has_previous_page,
    }))
}

async fn run_weekly_reset(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<WeeklyResetRequest>,
) -> ApiResult<Json<WeeklyResetResult>> {
    tracing::info!("Running weekly notification reset");

    let rows_affected = Member::reset_weekly_counts(&state.deps.db_pool).await?;

    tracing::info!(members_reset = rows_affected, "Weekly reset complete");

    Ok(Json(WeeklyResetResult {
        members_reset: rows_affected as i64,
    }))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Members/list", post(list))
        .route("/Members/run_weekly_reset", post(run_weekly_reset))
}
