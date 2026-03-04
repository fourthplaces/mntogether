use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::{AdminUser, OptionalUser};
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::domains::member::activities;
use crate::domains::member::models::member::Member;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct RegisterMemberRequest {
    pub expo_push_token: String,
    pub searchable_text: String,
    pub city: String,
    pub state: String,
}

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

impl From<Member> for MemberResult {
    fn from(m: Member) -> Self {
        Self {
            id: m.id,
            searchable_text: m.searchable_text,
            location_name: m.location_name,
            active: m.active,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RegisterMemberResult {
    pub member_id: Uuid,
    pub embedding_generated: bool,
}

// =============================================================================
// MemberObject handlers
// =============================================================================

async fn get(
    State(state): State<AppState>,
    _user: OptionalUser,
    Path(member_id): Path<Uuid>,
) -> ApiResult<Json<MemberResult>> {
    let member = Member::find_by_id(member_id, &state.deps.db_pool)
        .await
        .map_err(|e| ApiError::NotFound(format!("Member not found: {}", e)))?;

    Ok(Json(MemberResult::from(member)))
}

async fn update_status(
    State(state): State<AppState>,
    _user: AdminUser,
    Path(member_id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> ApiResult<Json<MemberResult>> {
    activities::update_member_status(member_id, req.active, &state.deps).await?;

    let member = Member::find_by_id(member_id, &state.deps.db_pool)
        .await
        .map_err(|e| ApiError::NotFound(format!("Member not found: {}", e)))?;

    Ok(Json(MemberResult::from(member)))
}

// =============================================================================
// RegisterMemberWorkflow handler
// =============================================================================

async fn register_member(
    State(state): State<AppState>,
    Path(_key): Path<String>,
    Json(req): Json<RegisterMemberRequest>,
) -> ApiResult<Json<RegisterMemberResult>> {
    tracing::info!(
        expo_push_token = %req.expo_push_token,
        city = %req.city,
        state = %req.state,
        "Starting register member"
    );

    // Step 1: Register member in DB (with geocoding) — must succeed
    let member_id = activities::register_member(
        req.expo_push_token,
        req.searchable_text,
        req.city,
        req.state,
        &state.deps,
    )
    .await?;

    // Step 2: Generate embedding — non-fatal, just log warning on failure
    let embedding_generated = match activities::generate_embedding(
        member_id,
        state.deps.embedding_service.as_ref(),
        &state.deps.db_pool,
    )
    .await
    {
        Ok(result) => {
            tracing::info!(
                member_id = %result.member_id,
                dimensions = result.dimensions,
                "Embedding generated for member"
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                member_id = %member_id,
                error = %e,
                "Failed to generate embedding (non-fatal)"
            );
            false
        }
    };

    Ok(Json(RegisterMemberResult {
        member_id,
        embedding_generated,
    }))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Member/{id}/get", post(get))
        .route("/Member/{id}/update_status", post(update_status))
        .route(
            "/RegisterMemberWorkflow/{key}/run",
            post(register_member),
        )
}
