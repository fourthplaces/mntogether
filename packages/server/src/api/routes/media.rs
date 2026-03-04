use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::ApiResult;
use crate::api::state::AppState;
use crate::domains::media::activities;
use crate::domains::media::models::Media;

// --- Request types ---

#[derive(Debug, Deserialize)]
pub struct PresignedUploadRequest {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmUploadRequest {
    pub storage_key: String,
    pub public_url: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub alt_text: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ListMediaRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteMediaRequest {
    pub id: String,
}

// --- Response types ---

#[derive(Debug, Serialize)]
pub struct PresignedUploadResponse {
    pub upload_url: String,
    pub storage_key: String,
    pub public_url: String,
}

#[derive(Debug, Serialize)]
pub struct MediaResult {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub url: String,
    pub storage_key: String,
    pub alt_text: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub created_at: String,
}

impl From<Media> for MediaResult {
    fn from(m: Media) -> Self {
        Self {
            id: m.id.to_string(),
            filename: m.filename,
            content_type: m.content_type,
            size_bytes: m.size_bytes,
            url: m.url,
            storage_key: m.storage_key,
            alt_text: m.alt_text,
            width: m.width,
            height: m.height,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MediaListResult {
    pub media: Vec<MediaResult>,
    pub total_count: i64,
    pub has_next_page: bool,
}

// --- Handlers ---

async fn presigned_upload(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<PresignedUploadRequest>,
) -> ApiResult<Json<PresignedUploadResponse>> {
    let result = activities::presign_upload(&req.filename, &req.content_type, &state.deps).await?;

    Ok(Json(PresignedUploadResponse {
        upload_url: result.upload_url,
        storage_key: result.storage_key,
        public_url: result.public_url,
    }))
}

async fn confirm_upload(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<ConfirmUploadRequest>,
) -> ApiResult<Json<MediaResult>> {
    let member_id = Some(user.0.member_id.into_uuid());

    let media = activities::confirm_upload(
        &req.storage_key,
        &req.public_url,
        &req.filename,
        &req.content_type,
        req.size_bytes,
        req.alt_text.as_deref(),
        req.width,
        req.height,
        member_id,
        &state.deps,
    )
    .await?;

    Ok(Json(MediaResult::from(media)))
}

async fn list(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListMediaRequest>,
) -> ApiResult<Json<MediaListResult>> {
    let limit = req.limit.unwrap_or(20);
    let offset = req.offset.unwrap_or(0);

    let (items, total_count, has_next_page) =
        activities::list_media(req.content_type.as_deref(), limit, offset, &state.deps).await?;

    Ok(Json(MediaListResult {
        media: items.into_iter().map(MediaResult::from).collect(),
        total_count,
        has_next_page,
    }))
}

async fn delete(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteMediaRequest>,
) -> ApiResult<Json<bool>> {
    let id = Uuid::parse_str(&req.id)
        .map_err(|e| anyhow::anyhow!("Invalid media ID: {}", e))?;

    activities::delete_media(id, &state.deps).await?;

    Ok(Json(true))
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/MediaService/presigned_upload", post(presigned_upload))
        .route("/MediaService/confirm_upload", post(confirm_upload))
        .route("/MediaService/list", post(list))
        .route("/MediaService/delete", post(delete))
}
