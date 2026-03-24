// Domain functions for listing operations
//
// These functions contain the business logic for listing CRUD operations,
// separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use typed_builder::TypedBuilder;

use crate::common::{MemberId, PostId};
use crate::domains::contacts::Contact;
use crate::domains::posts::models::{CreatePost, Post, UpdatePostContent};
use uuid::Uuid;

/// Input for updating and approving a post
#[derive(Debug, Clone, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct UpdateAndApprovePost {
    pub post_id: PostId,
    #[builder(default)]
    pub title: Option<String>,
    #[builder(default)]
    pub body_raw: Option<String>,
    #[builder(default)]
    pub contact_info: Option<JsonValue>,
    #[builder(default)]
    pub is_urgent: Option<bool>,
    #[builder(default)]
    pub location: Option<String>,
}

/// Create a new listing
pub async fn create_post(
    member_id: Option<MemberId>,
    title: String,
    body_raw: String,
    contact_info: Option<JsonValue>,
    is_urgent: bool,
    location: Option<String>,
    ip_address: Option<String>,
    submission_type: String,
    source_type: Option<&str>,
    source_id: Option<Uuid>,
    pool: &PgPool,
) -> Result<Post> {
    // Log IP for spam tracking
    if let Some(ref ip) = ip_address {
        tracing::info!(ip_address = %ip, "Listing submitted from IP");
    }

    // Create listing using model method
    let post = Post::create(
        CreatePost::builder()
            .title(title)
            .body_raw(body_raw)
            .is_urgent(is_urgent)
            .location(location)
            .submission_type(Some(submission_type))
            .submitted_by_id(member_id.map(|m| m.into_uuid()))
            .build(),
        pool,
    )
    .await
    .context("Failed to create listing")?;

    // Link to source via post_sources
    if let (Some(st), Some(sid)) = (source_type, source_id) {
        use crate::domains::posts::models::PostSource;
        if let Err(e) = PostSource::create(post.id, st, sid, None, pool).await {
            tracing::warn!(
                post_id = %post.id,
                error = %e,
                "Failed to create post source link"
            );
        }
    }

    // Save contact info if provided
    if let Some(ref contact) = contact_info {
        if let Err(e) = Contact::create_from_json_for_post(post.id, contact, pool).await {
            tracing::warn!(
                post_id = %post.id,
                error = %e,
                "Failed to save contact info for user-submitted listing"
            );
        }
    }

    Ok(post)
}

/// Update listing status and return the appropriate status string
pub async fn update_post_status(post_id: PostId, status: String, pool: &PgPool) -> Result<String> {
    Post::update_status(post_id, &status, pool)
        .await
        .context("Failed to update listing status")?;

    Ok(status)
}

/// Update listing content and approve it
pub async fn update_and_approve_post(input: UpdateAndApprovePost, pool: &PgPool) -> Result<()> {
    // Update listing content
    Post::update_content(
        UpdatePostContent::builder()
            .id(input.post_id)
            .title(input.title)
            .body_raw(input.body_raw)
            .is_urgent(input.is_urgent)
            .location(input.location)
            .build(),
        pool,
    )
    .await
    .context("Failed to update listing content")?;

    // Update contact info if provided (replace existing)
    if let Some(ref contact) = input.contact_info {
        // Delete existing contacts first
        Contact::delete_all_for_post(input.post_id, pool).await?;
        // Create new contacts
        if let Err(e) = Contact::create_from_json_for_post(input.post_id, contact, pool).await {
            tracing::warn!(
                post_id = %input.post_id,
                error = %e,
                "Failed to update contact info"
            );
        }
    }

    // Set status to active
    Post::update_status(input.post_id, "active", pool)
        .await
        .context("Failed to approve listing")?;

    Ok(())
}

/// Expire a post
pub async fn expire_post(post_id: PostId, pool: &PgPool) -> Result<Post> {
    Post::update_status(post_id, "expired", pool)
        .await
        .context("Failed to expire post")
}

/// Archive a post
pub async fn archive_post(post_id: PostId, pool: &PgPool) -> Result<Post> {
    Post::update_status(post_id, "archived", pool)
        .await
        .context("Failed to archive post")
}

/// Increment post view count (analytics)
/// Note: View tracking not implemented for Post model
pub async fn increment_post_view(_post_id: PostId, _pool: &PgPool) -> Result<()> {
    // View counting not implemented for posts - would need to add view_count column
    Ok(())
}

/// Increment post click count (analytics)
/// Note: Click tracking not implemented for Post model
pub async fn increment_post_click(_post_id: PostId, _pool: &PgPool) -> Result<()> {
    // Click counting not implemented for posts - would need to add click_count column
    Ok(())
}

/// Delete a listing
pub async fn delete_post(post_id: PostId, pool: &PgPool) -> Result<()> {
    Post::delete(post_id, pool)
        .await
        .context("Failed to delete listing")
}
