//! Media service (stateless Restate service)
//!
//! Handles presigned uploads, upload confirmation, listing, and deletion.
//! All operations route through Restate for durability.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::domains::media::activities;
use crate::domains::media::models::Media;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUploadRequest {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
}
impl_restate_serde!(PresignedUploadRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUploadResponse {
    pub upload_url: String,
    pub storage_key: String,
    pub public_url: String,
}
impl_restate_serde!(PresignedUploadResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
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
impl_restate_serde!(ConfirmUploadRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
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
impl_restate_serde!(MediaResult);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMediaRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub content_type: Option<String>,
}
impl_restate_serde!(ListMediaRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaListResult {
    pub media: Vec<MediaResult>,
    pub total_count: i64,
    pub has_next_page: bool,
}
impl_restate_serde!(MediaListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMediaRequest {
    pub id: String,
}
impl_restate_serde!(DeleteMediaRequest);

// =============================================================================
// Service trait
// =============================================================================

#[restate_sdk::service]
pub trait MediaService {
    async fn presigned_upload(
        req: PresignedUploadRequest,
    ) -> Result<PresignedUploadResponse, HandlerError>;

    async fn confirm_upload(req: ConfirmUploadRequest) -> Result<MediaResult, HandlerError>;

    async fn list(req: ListMediaRequest) -> Result<MediaListResult, HandlerError>;

    async fn delete(req: DeleteMediaRequest) -> Result<bool, HandlerError>;
}

// =============================================================================
// Implementation
// =============================================================================

pub struct MediaServiceImpl {
    deps: Arc<ServerDeps>,
}

impl MediaServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl MediaService for MediaServiceImpl {
    async fn presigned_upload(
        &self,
        ctx: Context<'_>,
        req: PresignedUploadRequest,
    ) -> Result<PresignedUploadResponse, HandlerError> {
        let _auth = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let deps = self.deps.clone();
        let filename = req.filename.clone();
        let content_type = req.content_type.clone();

        let result = ctx
            .run(|| async move {
                let r = activities::presign_upload(&filename, &content_type, &deps)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(PresignedUploadResponse {
                    upload_url: r.upload_url,
                    storage_key: r.storage_key,
                    public_url: r.public_url,
                })
            })
            .await?;

        Ok(result)
    }

    async fn confirm_upload(
        &self,
        ctx: Context<'_>,
        req: ConfirmUploadRequest,
    ) -> Result<MediaResult, HandlerError> {
        let auth = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let member_id = Some(auth.member_id.into_uuid());

        let deps = self.deps.clone();
        let storage_key = req.storage_key.clone();
        let public_url = req.public_url.clone();
        let filename = req.filename.clone();
        let content_type = req.content_type.clone();
        let size_bytes = req.size_bytes;
        let alt_text = req.alt_text.clone();
        let width = req.width;
        let height = req.height;

        let result = ctx
            .run(|| async move {
                let media = activities::confirm_upload(
                    &storage_key,
                    &public_url,
                    &filename,
                    &content_type,
                    size_bytes,
                    alt_text.as_deref(),
                    width,
                    height,
                    member_id,
                    &deps,
                )
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(MediaResult::from(media))
            })
            .await?;

        Ok(result)
    }

    async fn list(
        &self,
        ctx: Context<'_>,
        req: ListMediaRequest,
    ) -> Result<MediaListResult, HandlerError> {
        let _auth = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let deps = self.deps.clone();
        let content_type = req.content_type.clone();
        let limit = req.limit.unwrap_or(20);
        let offset = req.offset.unwrap_or(0);

        let result = ctx
            .run(|| async move {
                let (items, total_count, has_next_page) =
                    activities::list_media(content_type.as_deref(), limit, offset, &deps)
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(MediaListResult {
                    media: items.into_iter().map(MediaResult::from).collect(),
                    total_count,
                    has_next_page,
                })
            })
            .await?;

        Ok(result)
    }

    async fn delete(
        &self,
        ctx: Context<'_>,
        req: DeleteMediaRequest,
    ) -> Result<bool, HandlerError> {
        let _auth = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let deps = self.deps.clone();
        let id_str = req.id.clone();

        let result = ctx
            .run(|| async move {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| TerminalError::new(format!("Invalid media ID: {}", e)))?;
                activities::delete_media(id, &deps)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(true)
            })
            .await?;

        Ok(result)
    }
}
