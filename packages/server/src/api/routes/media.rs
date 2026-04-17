use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::ApiResult;
use crate::api::state::AppState;
use crate::domains::media::activities;
use crate::domains::media::models::{Media, MediaReference, MediaUsage};

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
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub unused_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeleteMediaRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMediaMetadataRequest {
    pub id: String,
    #[serde(default)]
    pub alt_text: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListUsageRequest {
    pub media_id: String,
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
    pub updated_at: String,
    /// Number of media_references pointing at this media. Populated on list
    /// (JOINed at query time); for single-row reads (confirm_upload, update
    /// metadata), starts at 0 — callers who need the fresh count should
    /// re-fetch via the list query.
    #[serde(default)]
    pub usage_count: i64,
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
            updated_at: m.updated_at.to_rfc3339(),
            usage_count: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MediaListResult {
    pub media: Vec<MediaResult>,
    pub total_count: i64,
    pub has_next_page: bool,
}

#[derive(Debug, Serialize)]
pub struct MediaUsageResult {
    pub referenceable_type: String,
    pub referenceable_id: String,
    pub field_key: Option<String>,
    pub title: String,
}

impl From<MediaUsage> for MediaUsageResult {
    fn from(u: MediaUsage) -> Self {
        Self {
            referenceable_type: u.referenceable_type,
            referenceable_id: u.referenceable_id.to_string(),
            field_key: u.field_key,
            title: u.title,
        }
    }
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
    let limit = req.limit.unwrap_or(24);
    let offset = req.offset.unwrap_or(0);

    let filters = crate::domains::media::models::media::MediaFilters {
        content_type_prefix: req.content_type.as_deref(),
        search: req.search.as_deref(),
        unused_only: req.unused_only,
    };
    let (items, total_count) =
        Media::list_with_usage(&filters, limit, offset, &state.deps.db_pool).await?;
    let has_next_page = offset + limit < total_count;

    let media: Vec<MediaResult> = items
        .into_iter()
        .map(|m| MediaResult {
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
            updated_at: m.updated_at.to_rfc3339(),
            usage_count: m.usage_count,
        })
        .collect();

    Ok(Json(MediaListResult {
        media,
        total_count,
        has_next_page,
    }))
}

async fn list_usage(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListUsageRequest>,
) -> ApiResult<Json<Vec<MediaUsageResult>>> {
    let media_id = Uuid::parse_str(&req.media_id)
        .map_err(|e| anyhow::anyhow!("Invalid media ID: {}", e))?;

    let usages = MediaReference::list_usage(media_id, &state.deps.db_pool).await?;
    Ok(Json(usages.into_iter().map(MediaUsageResult::from).collect()))
}

async fn update_metadata(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateMediaMetadataRequest>,
) -> ApiResult<Json<MediaResult>> {
    let id = Uuid::parse_str(&req.id)
        .map_err(|e| anyhow::anyhow!("Invalid media ID: {}", e))?;

    let media = Media::update_metadata(
        id,
        req.alt_text.as_deref(),
        req.filename.as_deref(),
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(MediaResult::from(media)))
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
        .route("/MediaService/list_usage", post(list_usage))
        .route("/MediaService/update_metadata", post(update_metadata))
        .route("/MediaService/delete", post(delete))
}
