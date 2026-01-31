// Domain functions for listing operations
//
// These functions contain the business logic for listing CRUD operations,
// separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::common::{ListingId, MemberId, PostId, WebsiteId};
use crate::domains::listings::models::{listing::Listing, ListingContact, ListingStatus};
use crate::domains::organization::models::post::Post;
use crate::domains::organization::utils::generate_tldr;
use crate::kernel::BaseAI;

/// Create a new listing with generated content hash and TLDR
pub async fn create_listing(
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
) -> Result<Listing> {
    // Log IP for spam tracking
    if let Some(ref ip) = ip_address {
        tracing::info!(ip_address = %ip, "Listing submitted from IP");
    }

    // Generate TLDR using AI
    let tldr = super::listing_extraction::generate_summary(ai, &description)
        .await
        .unwrap_or_else(|_| {
            // Fallback to truncation if AI fails
            generate_tldr(&description, 100)
        });

    // Create listing using model method
    let listing = Listing::create(
        organization_name,
        title,
        description,
        Some(tldr),
        "opportunity".to_string(),     // Default type
        "general".to_string(),         // Default category
        Some("accepting".to_string()), // Default capacity status
        urgency,
        location,
        ListingStatus::PendingApproval.to_string(),
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
        if let Err(e) = ListingContact::create_from_json(listing.id, contact, pool).await {
            tracing::warn!(
                listing_id = %listing.id,
                error = %e,
                "Failed to save contact info for user-submitted listing"
            );
        }
    }

    Ok(listing)
}

/// Update listing status and return the appropriate status string
pub async fn update_listing_status(
    listing_id: ListingId,
    status: String,
    pool: &PgPool,
) -> Result<String> {
    Listing::update_status(listing_id, &status, pool)
        .await
        .context("Failed to update listing status")?;

    Ok(status)
}

/// Update listing content and approve it
pub async fn update_and_approve_listing(
    listing_id: ListingId,
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
    Listing::update_content(
        listing_id,
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
        ListingContact::delete_all_for_listing(listing_id, pool).await?;
        // Create new contacts
        if let Err(e) = ListingContact::create_from_json(listing_id, contact, pool).await {
            tracing::warn!(
                listing_id = %listing_id,
                error = %e,
                "Failed to update contact info"
            );
        }
    }

    // Set status to active
    Listing::update_status(listing_id, "active", pool)
        .await
        .context("Failed to approve listing")?;

    Ok(())
}

/// Create a post for a listing and generate AI outreach copy
pub async fn create_post_for_listing(
    listing_id: ListingId,
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
            listing_id,
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
        Post::create_and_publish(listing_id, created_by, expires_in_days, pool)
            .await
            .context("Failed to create post")?
    };

    // Generate AI outreach copy for the post
    let listing = Listing::find_by_id(listing_id, pool)
        .await
        .context("Failed to find listing")?;

    // Use custom content if provided, otherwise use listing content
    let title_to_use = custom_title.as_ref().unwrap_or(&listing.title);
    let description_to_use = custom_description.as_ref().unwrap_or(&listing.description);

    // Extract contact email from listing's contact_info JSON
    let contact_email = None; // Listings don't have contact_info in the same way

    // Generate outreach copy using AI
    match super::listing_extraction::generate_outreach_copy(
        ai,
        &listing.organization_name,
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

/// Generate embedding for a listing
pub async fn generate_listing_embedding(
    listing_id: ListingId,
    embedding_service: &dyn crate::kernel::BaseEmbeddingService,
    pool: &PgPool,
) -> Result<usize> {
    // Get listing from database
    let listing = Listing::find_by_id(listing_id, pool)
        .await
        .context("Failed to find listing")?;

    // Generate embedding from description
    let embedding = embedding_service
        .generate(&listing.description)
        .await
        .context("Embedding generation failed")?;

    let dimensions = embedding.len();

    // Update listing with embedding
    Listing::update_embedding(listing_id, &embedding, pool)
        .await
        .context("Failed to save embedding")?;

    Ok(dimensions)
}

/// Create a custom post with admin-provided content
pub async fn create_custom_post(
    listing_id: ListingId,
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
        listing_id,
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
    let listing = Listing::find_by_id(listing_id, pool)
        .await
        .context("Failed to find listing")?;

    // Use custom content if provided, otherwise use listing content
    let title_to_use = custom_title.as_ref().unwrap_or(&listing.title);
    let description_to_use = custom_description.as_ref().unwrap_or(&listing.description);

    // Extract contact email from listing's contact_info JSON
    let contact_email = None; // Listings don't have contact_info in the same way

    // Generate outreach copy using AI
    match super::listing_extraction::generate_outreach_copy(
        ai,
        &listing.organization_name,
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

/// Delete a listing
pub async fn delete_listing(listing_id: ListingId, pool: &PgPool) -> Result<()> {
    Listing::delete(listing_id, pool)
        .await
        .context("Failed to delete listing")
}
