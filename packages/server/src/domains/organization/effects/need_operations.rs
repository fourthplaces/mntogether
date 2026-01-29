// Domain functions for need operations
//
// These functions contain the business logic for need CRUD operations,
// separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::common::{MemberId, NeedId, PostId, SourceId};
use crate::domains::organization::models::{need::OrganizationNeed, post::Post, NeedStatus};
use crate::domains::organization::utils::{generate_need_content_hash, generate_tldr};
use crate::kernel::BaseAI;

/// Create a new need with generated content hash and TLDR
pub async fn create_need(
    member_id: Option<MemberId>,
    organization_name: String,
    title: String,
    description: String,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    ip_address: Option<String>,
    submission_type: String,
    source_id: Option<SourceId>,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<OrganizationNeed> {
    // Generate content hash for deduplication
    let content_hash = generate_need_content_hash(&title, &description, &organization_name);

    // Generate TLDR using AI
    let tldr = super::need_extraction::generate_summary(ai, &description)
        .await
        .unwrap_or_else(|_| {
            // Fallback to truncation if AI fails
            generate_tldr(&description, 100)
        });

    // Create need using model method
    let need = OrganizationNeed::create(
        organization_name,
        title,
        description,
        tldr,
        contact_info,
        urgency,
        location,
        NeedStatus::PendingApproval.to_string(),
        content_hash,
        Some(submission_type),
        member_id,
        ip_address,
        source_id,
        None, // source_url - not applicable for user-submitted needs
        pool,
    )
    .await
    .context("Failed to create need")?;

    Ok(need)
}

/// Update need status and return the appropriate status string
pub async fn update_need_status(need_id: NeedId, status: String, pool: &PgPool) -> Result<String> {
    OrganizationNeed::update_status(need_id, &status, pool)
        .await
        .context("Failed to update need status")?;

    Ok(status)
}

/// Update need content and approve it
pub async fn update_and_approve_need(
    need_id: NeedId,
    title: Option<String>,
    description: Option<String>,
    description_markdown: Option<String>,
    tldr: Option<String>,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    pool: &PgPool,
) -> Result<()> {
    // Update need content
    OrganizationNeed::update_content(
        need_id,
        title,
        description,
        description_markdown,
        tldr,
        contact_info,
        urgency,
        location,
        pool,
    )
    .await
    .context("Failed to update need content")?;

    // Set status to active
    OrganizationNeed::update_status(need_id, "active", pool)
        .await
        .context("Failed to approve need")?;

    Ok(())
}

/// Create a post for a need and generate AI outreach copy
pub async fn create_post_for_need(
    need_id: NeedId,
    created_by: Option<MemberId>,
    custom_title: Option<String>,
    custom_description: Option<String>,
    expires_in_days: Option<i64>,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<Post> {
    // Create and publish post
    let mut post = if custom_title.is_some() || custom_description.is_some() {
        Post::create_and_publish_custom(
            need_id,
            created_by,
            custom_title.clone(),
            custom_description.clone(),
            None, // custom_tldr
            None, // targeting_hints
            expires_in_days,
            pool,
        )
        .await
        .context("Failed to create custom post")?
    } else {
        Post::create_and_publish(need_id, created_by, expires_in_days, pool)
            .await
            .context("Failed to create post")?
    };

    // Generate AI outreach copy for the post
    let need = OrganizationNeed::find_by_id(need_id, pool)
        .await
        .context("Failed to find need")?;

    // Use custom content if provided, otherwise use need content
    let title_to_use = custom_title.as_ref().unwrap_or(&need.title);
    let description_to_use = custom_description.as_ref().unwrap_or(&need.description);

    // Extract contact email from need's contact_info JSON
    let contact_email = need
        .contact_info
        .as_ref()
        .and_then(|info| info.get("email"))
        .and_then(|email| email.as_str());

    // Generate outreach copy using AI
    match super::need_extraction::generate_outreach_copy(
        ai,
        &need.organization_name,
        title_to_use,
        description_to_use,
        contact_email,
    )
    .await
    {
        Ok(outreach_copy) => {
            // Update post with generated outreach copy
            post = Post::update_outreach_copy(post.id, outreach_copy, pool)
                .await
                .context("Failed to update post with outreach copy")?;
            tracing::info!(post_id = %post.id, "Generated outreach copy for post");
        }
        Err(e) => {
            // Log error but don't fail the post creation
            tracing::warn!(
                post_id = %post.id,
                error = %e,
                "Failed to generate outreach copy, post created without it"
            );
        }
    }

    Ok(post)
}

/// Generate embedding for a need
pub async fn generate_need_embedding(
    need_id: NeedId,
    embedding_service: &dyn crate::kernel::BaseEmbeddingService,
    pool: &PgPool,
) -> Result<usize> {
    // Get need from database
    let need = OrganizationNeed::find_by_id(need_id, pool)
        .await
        .context("Failed to find need")?;

    // Generate embedding from description
    let embedding = embedding_service
        .generate(&need.description)
        .await
        .context("Embedding generation failed")?;

    let dimensions = embedding.len();

    // Update need with embedding
    OrganizationNeed::update_embedding(need_id, &embedding, pool)
        .await
        .context("Failed to save embedding")?;

    Ok(dimensions)
}

/// Create a custom post with admin-provided content
pub async fn create_custom_post(
    need_id: NeedId,
    created_by: Option<MemberId>,
    custom_title: Option<String>,
    custom_description: Option<String>,
    custom_tldr: Option<String>,
    targeting_hints: Option<serde_json::Value>,
    expires_in_days: Option<i64>,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<Post> {
    // Create and publish custom post
    let mut post = Post::create_and_publish_custom(
        need_id,
        created_by,
        custom_title.clone(),
        custom_description.clone(),
        custom_tldr,
        targeting_hints,
        expires_in_days,
        pool,
    )
    .await
    .context("Failed to create custom post")?;

    // Generate AI outreach copy for the post
    let need = OrganizationNeed::find_by_id(need_id, pool)
        .await
        .context("Failed to find need")?;

    // Use custom content if provided, otherwise use need content
    let title_to_use = custom_title.as_ref().unwrap_or(&need.title);
    let description_to_use = custom_description.as_ref().unwrap_or(&need.description);

    // Extract contact email from need's contact_info JSON
    let contact_email = need
        .contact_info
        .as_ref()
        .and_then(|info| info.get("email"))
        .and_then(|email| email.as_str());

    // Generate outreach copy using AI
    match super::need_extraction::generate_outreach_copy(
        ai,
        &need.organization_name,
        title_to_use,
        description_to_use,
        contact_email,
    )
    .await
    {
        Ok(outreach_copy) => {
            // Update post with generated outreach copy
            post = Post::update_outreach_copy(post.id, outreach_copy, pool)
                .await
                .context("Failed to update post with outreach copy")?;
            tracing::info!(post_id = %post.id, "Generated outreach copy for custom post");
        }
        Err(e) => {
            // Log error but don't fail the post creation
            tracing::warn!(
                post_id = %post.id,
                error = %e,
                "Failed to generate outreach copy, custom post created without it"
            );
        }
    }

    Ok(post)
}

/// Expire a post
pub async fn expire_post(post_id: PostId, pool: &PgPool) -> Result<Post> {
    Post::expire(post_id, pool)
        .await
        .context("Failed to expire post")
}

/// Archive a post
pub async fn archive_post(post_id: PostId, pool: &PgPool) -> Result<Post> {
    Post::archive(post_id, pool)
        .await
        .context("Failed to archive post")
}

/// Increment post view count (analytics)
pub async fn increment_post_view(post_id: PostId, pool: &PgPool) -> Result<()> {
    Post::increment_view_count(post_id, pool)
        .await
        .context("Failed to increment view count")
}

/// Increment post click count (analytics)
pub async fn increment_post_click(post_id: PostId, pool: &PgPool) -> Result<()> {
    Post::increment_click_count(post_id, pool)
        .await
        .context("Failed to increment click count")
}
