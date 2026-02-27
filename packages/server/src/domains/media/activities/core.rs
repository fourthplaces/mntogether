//! Media activities — entry-point functions for media operations.
//!
//! Called from Restate service handlers. Activities are pure functions
//! that take `&ServerDeps` explicitly.

use anyhow::Result;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::domains::media::models::media::{Media, MediaFilters};
use crate::kernel::ServerDeps;

/// Result of requesting a presigned upload URL.
pub struct PresignedUploadResult {
    pub upload_url: String,
    pub storage_key: String,
    pub public_url: String,
}

/// Request a presigned upload URL for the browser to PUT a file directly to S3.
/// Generates a unique storage key based on date + UUID.
pub async fn presign_upload(
    filename: &str,
    content_type: &str,
    deps: &ServerDeps,
) -> Result<PresignedUploadResult> {
    let storage = deps
        .storage
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Storage service not configured"))?;

    // Generate a unique storage key: media/YYYY/MM/{uuid}.{ext}
    let now = Utc::now();
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("bin")
        .to_lowercase();
    let key = format!(
        "media/{}/{:02}/{}.{}",
        now.format("%Y"),
        now.format("%m"),
        Uuid::new_v4(),
        ext
    );

    info!(filename = %filename, key = %key, "Generating presigned upload URL");

    let upload_url = storage
        .presigned_upload_url(&key, content_type, 3600) // 1 hour expiry
        .await?;
    let public_url = storage.public_url(&key);

    Ok(PresignedUploadResult {
        upload_url,
        storage_key: key,
        public_url,
    })
}

/// Confirm an upload: create the Media record in the database.
/// Called after the browser has successfully PUT the file to S3.
pub async fn confirm_upload(
    storage_key: &str,
    public_url: &str,
    filename: &str,
    content_type: &str,
    size_bytes: i64,
    alt_text: Option<&str>,
    width: Option<i32>,
    height: Option<i32>,
    uploaded_by: Option<Uuid>,
    deps: &ServerDeps,
) -> Result<Media> {
    info!(storage_key = %storage_key, filename = %filename, "Confirming upload");

    let media = Media::create(
        filename,
        content_type,
        size_bytes,
        storage_key,
        public_url,
        alt_text,
        width,
        height,
        uploaded_by,
        &deps.db_pool,
    )
    .await?;

    Ok(media)
}

/// Delete a media item — remove from S3 and from the database.
pub async fn delete_media(media_id: Uuid, deps: &ServerDeps) -> Result<()> {
    let media = Media::find_by_id(media_id, &deps.db_pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Media not found: {}", media_id))?;

    info!(media_id = %media_id, key = %media.storage_key, "Deleting media");

    // Delete from object storage first
    if let Some(storage) = deps.storage.as_ref() {
        storage.delete(&media.storage_key).await?;
    }

    // Then delete the DB record
    Media::delete(media_id, &deps.db_pool).await?;

    Ok(())
}

/// List media with pagination and optional content-type filter.
pub async fn list_media(
    content_type_prefix: Option<&str>,
    limit: i64,
    offset: i64,
    deps: &ServerDeps,
) -> Result<(Vec<Media>, i64, bool)> {
    let filters = MediaFilters {
        content_type_prefix,
    };
    let (items, total_count) =
        Media::list_paginated(&filters, limit, offset, &deps.db_pool).await?;
    let has_next_page = offset + limit < total_count;
    Ok((items, total_count, has_next_page))
}
