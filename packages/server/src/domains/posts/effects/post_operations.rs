// Domain functions for listing operations
//
// These functions contain the business logic for listing CRUD operations,
// separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::common::{MemberId, PostId, WebsiteId};
use crate::domains::organization::utils::generate_tldr;
use crate::domains::posts::models::{Post, PostContact, PostStatus};
use crate::kernel::BaseAI;

/// Create a new listing with generated content hash and TLDR
pub async fn create_post(
    _member_id: Option<MemberId>, // TODO: Store submitted_by_member_id for tracking
    organization_name: String,
    title: String,
    description: String,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    ip_address: Option<String>,
    submission_type: String,
    website_id: Option<WebsiteId>,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<Post> {
    // Log IP for spam tracking
    if let Some(ref ip) = ip_address {
        tracing::info!(ip_address = %ip, "Listing submitted from IP");
    }

    // Generate TLDR using AI
    let tldr = super::post_extraction::generate_summary(ai, &description)
        .await
        .unwrap_or_else(|_| {
            // Fallback to truncation if AI fails
            generate_tldr(&description, 100)
        });

    // Create listing using model method
    let post = Post::create(
        organization_name,
        title,
        description,
        Some(tldr),
        "opportunity".to_string(),     // Default type
        "general".to_string(),         // Default category
        Some("accepting".to_string()), // Default capacity status
        urgency,
        location,
        PostStatus::PendingApproval.to_string(),
        "en".to_string(), // Default language
        Some(submission_type),
        None, // submitted_by_admin_id
        website_id,
        None, // source_url - not applicable for user-submitted listings
        None, // organization_id
        pool,
    )
    .await
    .context("Failed to create listing")?;

    // Save contact info if provided
    if let Some(ref contact) = contact_info {
        if let Err(e) = PostContact::create_from_json(post.id, contact, pool).await {
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
pub async fn update_and_approve_post(
    post_id: PostId,
    title: Option<String>,
    description: Option<String>,
    description_markdown: Option<String>,
    tldr: Option<String>,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    pool: &PgPool,
) -> Result<()> {
    // Update listing content
    Post::update_content(
        post_id,
        title,
        description,
        description_markdown,
        tldr,
        None, // category
        urgency,
        location,
        pool,
    )
    .await
    .context("Failed to update listing content")?;

    // Update contact info if provided (replace existing)
    if let Some(ref contact) = contact_info {
        // Delete existing contacts first
        PostContact::delete_all_for_post(post_id, pool).await?;
        // Create new contacts
        if let Err(e) = PostContact::create_from_json(post_id, contact, pool).await {
            tracing::warn!(
                post_id = %post_id,
                error = %e,
                "Failed to update contact info"
            );
        }
    }

    // Set status to active
    Post::update_status(post_id, "active", pool)
        .await
        .context("Failed to approve listing")?;

    Ok(())
}

/// Create a post for a listing and generate AI outreach copy
/// Note: This function is deprecated - the announcement model was removed
pub async fn create_post_for_post(
    post_id: PostId,
    _created_by: Option<MemberId>,
    _custom_title: Option<String>,
    _custom_description: Option<String>,
    _expires_in_days: Option<i64>,
    _ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<Post> {
    // The announcement system was removed, so we just return the post itself
    Post::find_by_id(post_id, pool)
        .await
        .context("Failed to find listing")?
        .ok_or_else(|| anyhow::anyhow!("Listing not found"))
}

/// Generate embedding for a listing
/// NOTE: Embeddings are no longer used for deduplication. This function is deprecated
/// and returns 0 for backwards compatibility.
#[allow(unused_variables)]
pub async fn generate_post_embedding(
    post_id: PostId,
    embedding_service: &dyn crate::kernel::BaseEmbeddingService,
    pool: &PgPool,
) -> Result<usize> {
    // Embeddings are no longer used - LLM-based deduplication handles this now
    Ok(0)
}

/// Create a custom post with admin-provided content
/// Note: This function is deprecated - the announcement model was removed
pub async fn create_custom_post(
    post_id: PostId,
    _created_by: Option<MemberId>,
    _custom_title: Option<String>,
    _custom_description: Option<String>,
    _custom_tldr: Option<String>,
    _targeting_hints: Option<serde_json::Value>,
    _expires_in_days: Option<i64>,
    _ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<Post> {
    // The announcement system was removed, so we just return the post itself
    Post::find_by_id(post_id, pool)
        .await
        .context("Failed to find listing")?
        .ok_or_else(|| anyhow::anyhow!("Listing not found"))
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
