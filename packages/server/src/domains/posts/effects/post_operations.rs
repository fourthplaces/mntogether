// Domain functions for listing operations
//
// These functions contain the business logic for listing CRUD operations,
// separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use openai_client::OpenAIClient;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use typed_builder::TypedBuilder;

use crate::common::{MemberId, PostId, WebsiteId};
use crate::common::utils::generate_tldr;
use crate::domains::contacts::Contact;
use crate::domains::posts::models::{CreatePost, Post, UpdatePostContent};

/// Input for updating and approving a post
#[derive(Debug, Clone, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct UpdateAndApprovePost {
    pub post_id: PostId,
    #[builder(default)]
    pub title: Option<String>,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub description_markdown: Option<String>,
    #[builder(default)]
    pub tldr: Option<String>,
    #[builder(default)]
    pub contact_info: Option<JsonValue>,
    #[builder(default)]
    pub urgency: Option<String>,
    #[builder(default)]
    pub location: Option<String>,
}

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
    ai: &OpenAIClient,
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
        CreatePost::builder()
            .organization_name(organization_name)
            .title(title)
            .description(description)
            .tldr(Some(tldr))
            .capacity_status(Some("accepting".to_string()))
            .urgency(urgency)
            .location(location)
            .submission_type(Some(submission_type))
            .website_id(website_id)
            .build(),
        pool,
    )
    .await
    .context("Failed to create listing")?;

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
            .description(input.description)
            .description_markdown(input.description_markdown)
            .tldr(input.tldr)
            .urgency(input.urgency)
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

/// Generate embedding for a post (for semantic search)
pub async fn generate_post_embedding(
    post_id: PostId,
    embedding_service: &dyn crate::kernel::BaseEmbeddingService,
    pool: &PgPool,
) -> Result<usize> {
    let post = Post::find_by_id(post_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    let embedding_text = post.get_embedding_text();
    let embedding = embedding_service.generate(&embedding_text).await?;
    let dimensions = embedding.len();

    Post::update_embedding(post_id, &embedding, pool).await?;

    Ok(dimensions)
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
