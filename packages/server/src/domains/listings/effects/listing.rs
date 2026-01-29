use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use serde_json::Value as JsonValue;
use tracing::info;

use super::deps::ServerDeps;
use crate::common::auth::{Actor, AdminCapability, AuthError};
use crate::common::{DomainId, ExtractedListing, JobId, ListingId, MemberId, PostId, SourceId};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;

/// Listing Effect - Handles CreateListing, UpdateListingStatus, UpdateListingAndApprove, CreatePost, GenerateListingEmbedding commands
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct ListingEffect;

#[async_trait]
impl Effect<ListingCommand, ServerDeps> for ListingEffect {
    type Event = ListingEvent;

    async fn execute(
        &self,
        cmd: ListingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ListingEvent> {
        match cmd {
            ListingCommand::CreateListing {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
                submission_type,
            } => {
                handle_create_listing(
                    member_id,
                    organization_name,
                    title,
                    description,
                    contact_info,
                    urgency,
                    location,
                    ip_address,
                    submission_type,
                    &ctx,
                )
                .await
            }

            ListingCommand::UpdateListingStatus {
                listing_id,
                status,
                rejection_reason,
                requested_by,
                is_admin,
            } => {
                handle_update_listing_status(
                    listing_id,
                    status,
                    rejection_reason,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            ListingCommand::UpdateListingAndApprove {
                listing_id,
                title,
                description,
                description_markdown,
                tldr,
                contact_info,
                urgency,
                location,
                requested_by,
                is_admin,
            } => {
                handle_update_listing_and_approve(
                    listing_id,
                    title,
                    description,
                    description_markdown,
                    tldr,
                    contact_info,
                    urgency,
                    location,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            ListingCommand::CreatePost {
                listing_id,
                created_by,
                custom_title,
                custom_description,
                expires_in_days,
            } => {
                handle_create_post(
                    listing_id,
                    created_by,
                    custom_title,
                    custom_description,
                    expires_in_days,
                    &ctx,
                )
                .await
            }

            ListingCommand::GenerateListingEmbedding { listing_id } => {
                handle_generate_listing_embedding(listing_id, &ctx).await
            }

            ListingCommand::CreateCustomPost {
                listing_id,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                expires_in_days,
                created_by,
                requested_by,
                is_admin,
            } => {
                handle_create_custom_post(
                    listing_id,
                    custom_title,
                    custom_description,
                    custom_tldr,
                    targeting_hints,
                    expires_in_days,
                    created_by,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            ListingCommand::RepostListing {
                listing_id,
                created_by,
                requested_by,
                is_admin,
            } => handle_repost_listing(listing_id, created_by, requested_by, is_admin, &ctx).await,

            ListingCommand::ExpirePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_expire_post(post_id, requested_by, is_admin, &ctx).await,

            ListingCommand::ArchivePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_archive_post(post_id, requested_by, is_admin, &ctx).await,

            ListingCommand::IncrementPostView { post_id } => {
                handle_increment_post_view(post_id, &ctx).await
            }

            ListingCommand::IncrementPostClick { post_id } => {
                handle_increment_post_click(post_id, &ctx).await
            }

            ListingCommand::DeleteListing {
                listing_id,
                requested_by,
                is_admin,
            } => handle_delete_listing(listing_id, requested_by, is_admin, &ctx).await,

            ListingCommand::AddScrapeUrl {
                source_id,
                url,
                requested_by,
                is_admin,
            } => handle_add_scrape_url(source_id, url, requested_by, is_admin, &ctx).await,

            ListingCommand::RemoveScrapeUrl {
                source_id,
                url,
                requested_by,
                is_admin,
            } => handle_remove_scrape_url(source_id, url, requested_by, is_admin, &ctx).await,

            ListingCommand::CreateListingsFromResourceLink {
                job_id,
                url,
                listings,
                context,
                submitter_contact,
            } => {
                handle_create_listings_from_resource_link(
                    job_id,
                    url,
                    listings,
                    context,
                    submitter_contact,
                    &ctx,
                )
                .await
            }

            ListingCommand::CreateOrganizationSourceFromLink {
                url,
                organization_name,
                submitter_contact,
            } => {
                handle_create_organization_source_from_link(
                    url,
                    organization_name,
                    submitter_contact,
                    &ctx,
                )
                .await
            }

            _ => anyhow::bail!("ListingEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Listing handlers
// ============================================================================

async fn handle_create_listing(
    member_id: Option<MemberId>,
    organization_name: String,
    title: String,
    description: String,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    ip_address: Option<String>,
    submission_type: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    let listing = super::listing_operations::create_listing(
        member_id,
        organization_name.clone(),
        title,
        description,
        contact_info,
        urgency,
        location,
        ip_address,
        submission_type.clone(),
        None, // domain_id
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(ListingEvent::ListingCreated {
        listing_id: listing.id,
        organization_name: listing.organization_name,
        title: listing.title,
        submission_type,
    })
}

async fn handle_update_listing_status(
    listing_id: ListingId,
    status: String,
    rejection_reason: Option<String>,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check - only admins can update listing status
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdateListingStatus".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let updated_status =
        super::listing_operations::update_listing_status(listing_id, status.clone(), &ctx.deps().db_pool)
            .await?;

    if updated_status == "active" {
        Ok(ListingEvent::ListingApproved { listing_id })
    } else if updated_status == "rejected" {
        Ok(ListingEvent::ListingRejected {
            listing_id,
            reason: rejection_reason.unwrap_or_else(|| "No reason provided".to_string()),
        })
    } else {
        Ok(ListingEvent::ListingUpdated { listing_id })
    }
}

async fn handle_update_listing_and_approve(
    listing_id: ListingId,
    title: Option<String>,
    description: Option<String>,
    description_markdown: Option<String>,
    tldr: Option<String>,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check - only admins can edit and approve listings
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdateListingAndApprove".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::listing_operations::update_and_approve_listing(
        listing_id,
        title,
        description,
        description_markdown,
        tldr,
        contact_info,
        urgency,
        location,
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(ListingEvent::ListingApproved { listing_id })
}

async fn handle_generate_listing_embedding(
    listing_id: ListingId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    match super::listing_operations::generate_listing_embedding(
        listing_id,
        ctx.deps().embedding_service.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(dimensions) => Ok(ListingEvent::ListingEmbeddingGenerated {
            listing_id,
            dimensions,
        }),
        Err(e) => Ok(ListingEvent::ListingEmbeddingFailed {
            listing_id,
            reason: e.to_string(),
        }),
    }
}

// ============================================================================
// Post handlers
// ============================================================================

async fn handle_create_post(
    listing_id: ListingId,
    created_by: Option<MemberId>,
    custom_title: Option<String>,
    custom_description: Option<String>,
    expires_in_days: Option<i64>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    let post = super::listing_operations::create_post_for_listing(
        listing_id,
        created_by,
        custom_title,
        custom_description,
        expires_in_days,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(ListingEvent::PostCreated {
        post_id: post.id,
        listing_id,
    })
}

async fn handle_create_custom_post(
    listing_id: ListingId,
    custom_title: Option<String>,
    custom_description: Option<String>,
    custom_tldr: Option<String>,
    targeting_hints: Option<JsonValue>,
    expires_in_days: Option<i64>,
    created_by: MemberId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check - only admins can create custom posts
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CreateCustomPost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let post = super::listing_operations::create_custom_post(
        listing_id,
        Some(created_by),
        custom_title,
        custom_description,
        custom_tldr,
        targeting_hints,
        expires_in_days,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(ListingEvent::PostCreated {
        post_id: post.id,
        listing_id,
    })
}

async fn handle_repost_listing(
    listing_id: ListingId,
    created_by: MemberId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check - only admins can repost listings
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RepostListing".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let post = super::listing_operations::create_post_for_listing(
        listing_id,
        Some(created_by),
        None,
        None,
        None,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(ListingEvent::PostCreated {
        post_id: post.id,
        listing_id,
    })
}

async fn handle_expire_post(
    post_id: PostId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check - only admins can expire posts
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ExpirePost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::listing_operations::expire_post(post_id, &ctx.deps().db_pool).await?;

    Ok(ListingEvent::PostExpired { post_id })
}

async fn handle_archive_post(
    post_id: PostId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check - only admins can archive posts
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ArchivePost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::listing_operations::archive_post(post_id, &ctx.deps().db_pool).await?;

    Ok(ListingEvent::PostArchived { post_id })
}

async fn handle_increment_post_view(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    super::listing_operations::increment_post_view(post_id, &ctx.deps().db_pool).await?;
    Ok(ListingEvent::PostViewed { post_id })
}

async fn handle_increment_post_click(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    super::listing_operations::increment_post_click(post_id, &ctx.deps().db_pool).await?;
    Ok(ListingEvent::PostClicked { post_id })
}

async fn handle_create_listings_from_resource_link(
    _job_id: JobId,
    url: String,
    listings: Vec<ExtractedListing>,
    context: Option<String>,
    _submitter_contact: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    use crate::domains::listings::models::Listing;
use crate::domains::organization::models::OrganizationSource;
    use tracing::info;

    // Find the organization source that was created during submission
    let organization_name = context.clone().unwrap_or_else(|| "Submitted Resource".to_string());

    // Find the source by URL (it was created in the mutation)
    let source = OrganizationSource::find_by_url(&url, &ctx.deps().db_pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Organization source not found for URL: {}", url))?;

    let source_id = source.id;

    // Update last_scraped_at now that we've processed it
    sqlx::query("UPDATE organization_sources SET last_scraped_at = NOW() WHERE id = $1")
        .bind(source_id.as_uuid())
        .execute(&ctx.deps().db_pool)
        .await?;

    // Create each extracted listing as a user_submitted listing in pending_approval status
    let mut created_count = 0;

    for extracted_listing in listings {
        let contact_json = extracted_listing
            .contact
            .and_then(|c| serde_json::to_value(c).ok());

        // Calculate content hash for deduplication
        let content_hash = {
            use sha2::{Digest, Sha256};
            let combined = format!("{}{}", extracted_listing.title, extracted_listing.description);
            let mut hasher = Sha256::new();
            hasher.update(combined.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        match Listing::create(
            organization_name.clone(),
            extracted_listing.title.clone(),
            extracted_listing.description.clone(),
            Some(extracted_listing.tldr),
            "opportunity".to_string(),
            "general".to_string(),
            Some("accepting".to_string()),
            extracted_listing.urgency,
            None, // location
            "pending_approval".to_string(),
            Some(content_hash),
            "en".to_string(), // source_language
            Some("user_submitted".to_string()),
            None, // submitted_by_admin_id
            Some(DomainId::from_uuid(source_id.into_uuid())),
            Some(url.clone()),
            None, // organization_id
            &ctx.deps().db_pool,
        ).await {
            Ok(listing) => {
                created_count += 1;
                info!(
                    listing_id = %listing.id,
                    org = %listing.organization_name,
                    title = %listing.title,
                    "Created listing from resource link"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    title = %extracted_listing.title,
                    "Failed to create listing from resource link"
                );
            }
        }
    }

    info!(created_count = %created_count, "Created listings from resource link");

    // Return a success event (we'll use ListingCreated for now, but could create a new event type)
    // For simplicity, just return a generic success event
    Ok(ListingEvent::ListingCreated {
        listing_id: crate::common::ListingId::new(), // Dummy ID
        organization_name: "Resource Link".to_string(),
        title: format!("{} listings created", created_count),
        submission_type: "user_submitted".to_string(),
    })
}

/// Extract domain from URL (e.g., "https://example.org/path" -> "example.org")
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|parsed| {
        parsed.host_str().map(|host| {
            // Remove www. prefix if present for consistent matching
            host.strip_prefix("www.").unwrap_or(host).to_lowercase()
        })
    })
}

async fn handle_create_organization_source_from_link(
    url: String,
    organization_name: String,
    submitter_contact: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    use crate::common::{JobId, SourceId};
    use crate::domains::organization::models::OrganizationSource;
    use tracing::info;

    // Validate URL format (moved from edge to effect)
    super::source_operations::validate_url(&url)?;

    // Generate a new job ID for tracking the scraping workflow
    let job_id = JobId::new();

    info!(
        url = %url,
        organization_name = %organization_name,
        job_id = %job_id,
        "Processing submitted resource link"
    );

    // Extract domain from submitted URL
    let domain = extract_domain(&url).ok_or_else(|| {
        anyhow::anyhow!("Invalid URL: could not extract domain")
    })?;

    info!(domain = %domain, "Extracted domain from URL");

    // Check if we already have a source for this domain
    let existing_sources = OrganizationSource::find_active(&ctx.deps().db_pool).await?;

    let matching_source = existing_sources.iter().find(|source| {
        if let Some(existing_domain) = extract_domain(&source.source_url) {
            existing_domain == domain
        } else {
            false
        }
    });

    let (source_id, event_type) = if let Some(existing) = matching_source {
        // Domain already exists - add URL to scrape_urls
        info!(
            source_id = %existing.id,
            existing_org = %existing.organization_name,
            "Found existing source for domain, adding URL to scrape_urls"
        );

        OrganizationSource::add_scrape_url(existing.id, url.clone(), &ctx.deps().db_pool).await?;

        (existing.id, "added_to_existing")
    } else {
        // New domain - create new source
        info!(domain = %domain, "No existing source found, creating new organization");

        let source_id = SourceId::new();
        let source = OrganizationSource {
            id: source_id,
            organization_name: organization_name.clone(),
            source_url: url.clone(),
            scrape_urls: None, // No specific URLs configured initially
            last_scraped_at: None,
            scrape_frequency_hours: 24, // Default to daily scrapes
            active: true,
            created_at: chrono::Utc::now(),
        };

        source.insert(&ctx.deps().db_pool).await?;

        (source_id, "created_new")
    };

    info!(
        source_id = %source_id,
        job_id = %job_id,
        event_type = %event_type,
        "Organization source processed successfully"
    );

    // Return event with job_id for tracking
    Ok(ListingEvent::OrganizationSourceCreatedFromLink {
        source_id,
        job_id,
        url,
        organization_name,
        submitter_contact,
    })
}

/// Handle DeleteListing command
async fn handle_delete_listing(
    listing_id: ListingId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Check authorization - only admins can delete listings
    Actor::new(requested_by)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
        .map_err(|e| {
            info!(
                user_id = %requested_by,
                action = "DeleteListing",
                error = ?e,
                "Authorization denied"
            );
            anyhow::anyhow!("Authorization denied: {:?}", e)
        })?;

    info!(listing_id = %listing_id, "Deleting listing");

    // Delete the listing
    super::listing_operations::delete_listing(listing_id, &ctx.deps().db_pool).await?;

    Ok(ListingEvent::ListingDeleted { listing_id })
}

/// Handle AddScrapeUrl command
async fn handle_add_scrape_url(
    source_id: SourceId,
    url: String,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Check authorization - only admins can manage scrape URLs
    Actor::new(requested_by)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
        .map_err(|e| {
            info!(
                user_id = %requested_by,
                action = "AddScrapeUrl",
                error = ?e,
                "Authorization denied"
            );
            anyhow::anyhow!("Authorization denied: {:?}", e)
        })?;

    info!(source_id = %source_id, url = %url, "Adding scrape URL");

    // Add the URL (includes validation)
    super::source_operations::add_scrape_url(source_id, url.clone(), &ctx.deps().db_pool).await?;

    Ok(ListingEvent::ScrapeUrlAdded { source_id, url })
}

/// Handle RemoveScrapeUrl command
async fn handle_remove_scrape_url(
    source_id: SourceId,
    url: String,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Check authorization - only admins can manage scrape URLs
    Actor::new(requested_by)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
        .map_err(|e| {
            info!(
                user_id = %requested_by,
                action = "RemoveScrapeUrl",
                error = ?e,
                "Authorization denied"
            );
            anyhow::anyhow!("Authorization denied: {:?}", e)
        })?;

    info!(source_id = %source_id, url = %url, "Removing scrape URL");

    // Remove the URL
    super::source_operations::remove_scrape_url(source_id, url.clone(), &ctx.deps().db_pool).await?;

    Ok(ListingEvent::ScrapeUrlRemoved { source_id, url })
}
