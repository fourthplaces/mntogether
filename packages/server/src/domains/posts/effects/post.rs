//! Post effect - handles post CRUD and lifecycle request events
//!
//! This effect is a thin orchestration layer that dispatches request events to handler functions.
//! Following CLAUDE.md: Effects must be thin orchestration layers, business logic in actions.

use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use serde_json::Value as JsonValue;
use tracing::{info, warn};

use crate::kernel::ServerDeps;
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{ExtractedPost, JobId, PostId, MemberId, WebsiteId};
use crate::domains::posts::events::PostEvent;

/// Post Effect - Handles post CRUD and lifecycle request events
///
/// This effect is a thin orchestration layer that dispatches events to handler functions.
pub struct PostEffect;

#[async_trait]
impl Effect<PostEvent, ServerDeps> for PostEffect {
    type Event = PostEvent;

    async fn handle(
        &mut self,
        event: PostEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<Option<PostEvent>> {
        match event {
            // =================================================================
            // Request Events → Dispatch to Handlers
            // =================================================================
            PostEvent::CreatePostEntryRequested {
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
                handle_create_post_entry(
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
                .map(Some)
            }

            PostEvent::SubmitListingRequested {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
            } => {
                handle_create_post_entry(
                    member_id,
                    organization_name,
                    title,
                    description,
                    contact_info,
                    urgency,
                    location,
                    ip_address,
                    "user_submitted".to_string(),
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::UpdatePostStatusRequested {
                post_id,
                status,
                rejection_reason,
                requested_by,
                is_admin,
            } => {
                handle_update_post_status(
                    post_id,
                    status,
                    rejection_reason,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::ApproveListingRequested {
                post_id,
                requested_by,
                is_admin,
            } => {
                handle_update_post_status(
                    post_id,
                    "active".to_string(),
                    None,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::RejectListingRequested {
                post_id,
                reason,
                requested_by,
                is_admin,
            } => {
                handle_update_post_status(
                    post_id,
                    "rejected".to_string(),
                    Some(reason),
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::EditAndApproveListingRequested {
                post_id,
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
                handle_update_post_and_approve(
                    post_id,
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
                .map(Some)
            }

            PostEvent::CreatePostRequested {
                post_id,
                created_by,
                custom_title,
                custom_description,
                expires_in_days,
            } => {
                handle_create_post_announcement(
                    post_id,
                    created_by,
                    custom_title,
                    custom_description,
                    expires_in_days,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::GeneratePostEmbeddingRequested { post_id } => {
                // Embeddings are no longer used - this is a no-op for backwards compatibility
                Ok(Some(PostEvent::PostEmbeddingGenerated { post_id, dimensions: 0 }))
            }

            PostEvent::CreateCustomPostRequested {
                post_id,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                expires_in_days,
                requested_by,
                is_admin,
            } => {
                handle_create_custom_post(
                    post_id,
                    custom_title,
                    custom_description,
                    custom_tldr,
                    targeting_hints,
                    expires_in_days,
                    requested_by,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::RepostPostRequested {
                post_id,
                requested_by,
                is_admin,
            } => handle_repost_post(post_id, requested_by, requested_by, is_admin, &ctx).await.map(Some),

            PostEvent::ExpirePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => handle_expire_post(post_id, requested_by, is_admin, &ctx).await.map(Some),

            PostEvent::ArchivePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => handle_archive_post(post_id, requested_by, is_admin, &ctx).await.map(Some),

            PostEvent::PostViewedRequested { post_id } => {
                handle_increment_post_view(post_id, &ctx).await.map(Some)
            }

            PostEvent::PostClickedRequested { post_id } => {
                handle_increment_post_click(post_id, &ctx).await.map(Some)
            }

            PostEvent::DeletePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => handle_delete_post(post_id, requested_by, is_admin, &ctx).await.map(Some),

            PostEvent::CreatePostsFromResourceLinkRequested {
                job_id,
                url,
                posts,
                context,
                submitter_contact,
            } => {
                handle_create_posts_from_resource_link(
                    job_id,
                    url,
                    posts,
                    context,
                    submitter_contact,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::CreateWebsiteFromLinkRequested {
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
                .map(Some)
            }

            PostEvent::SubmitResourceLinkRequested {
                url,
                context: _,
                submitter_contact,
            } => {
                // Extract organization name from URL
                let organization_name = url
                    .split("//")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("Unknown Organization")
                    .to_string();

                handle_create_organization_source_from_link(
                    url,
                    organization_name,
                    submitter_contact,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::ReportListingRequested {
                post_id,
                reported_by,
                reporter_email,
                reason,
                category,
            } => {
                super::post_report::handle_create_report(
                    post_id,
                    reported_by,
                    reporter_email,
                    reason,
                    category,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::ResolveReportRequested {
                report_id,
                resolved_by,
                resolution_notes,
                action_taken,
                is_admin,
            } => {
                super::post_report::handle_resolve_report(
                    report_id,
                    resolved_by,
                    resolution_notes,
                    action_taken,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::DismissReportRequested {
                report_id,
                resolved_by,
                resolution_notes,
                is_admin,
            } => {
                super::post_report::handle_dismiss_report(
                    report_id,
                    resolved_by,
                    resolution_notes,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            PostEvent::DeduplicatePostsRequested {
                job_id,
                similarity_threshold: _,
                requested_by,
                is_admin,
            } => {
                handle_deduplicate_posts(job_id, requested_by, is_admin, &ctx)
                    .await
                    .map(Some)
            }

            // =================================================================
            // Other Events → Terminal, no follow-up needed
            // =================================================================
            _ => Ok(None),
        }
    }
}

// ============================================================================
// Post Entry handlers (user submissions)
// ============================================================================

async fn handle_create_post_entry(
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
) -> Result<PostEvent> {
    let post = super::post_operations::create_post(
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

    Ok(PostEvent::PostEntryCreated {
        post_id: post.id,
        organization_name: post.organization_name,
        title: post.title,
        submission_type,
    })
}

async fn handle_update_post_status(
    post_id: PostId,
    status: String,
    rejection_reason: Option<String>,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    // Authorization check - only admins can update listing status
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdatePostStatus".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let updated_status = super::post_operations::update_post_status(
        post_id,
        status.clone(),
        &ctx.deps().db_pool,
    )
    .await?;

    if updated_status == "active" {
        Ok(PostEvent::PostApproved { post_id })
    } else if updated_status == "rejected" {
        Ok(PostEvent::PostRejected {
            post_id,
            reason: rejection_reason.unwrap_or_else(|| "No reason provided".to_string()),
        })
    } else {
        Ok(PostEvent::ListingUpdated { post_id })
    }
}

async fn handle_update_post_and_approve(
    post_id: PostId,
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
) -> Result<PostEvent> {
    // Authorization check - only admins can edit and approve listings
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdatePostAndApprove".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::post_operations::update_and_approve_post(
        post_id,
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

    Ok(PostEvent::PostApproved { post_id })
}

// ============================================================================
// Post announcement handlers
// ============================================================================

async fn handle_create_post_announcement(
    post_id: PostId,
    created_by: Option<MemberId>,
    custom_title: Option<String>,
    custom_description: Option<String>,
    expires_in_days: Option<i64>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    let post = super::post_operations::create_post_for_post(
        post_id,
        created_by,
        custom_title,
        custom_description,
        expires_in_days,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(PostEvent::PostCreated {
        post_id: post.id,
    })
}

async fn handle_create_custom_post(
    post_id: PostId,
    custom_title: Option<String>,
    custom_description: Option<String>,
    custom_tldr: Option<String>,
    targeting_hints: Option<JsonValue>,
    expires_in_days: Option<i64>,
    created_by: MemberId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    // Authorization check - only admins can create custom posts
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CreateCustomPost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let post = super::post_operations::create_custom_post(
        post_id,
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

    Ok(PostEvent::PostCreated {
        post_id: post.id,
    })
}

async fn handle_repost_post(
    post_id: PostId,
    created_by: MemberId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    // Authorization check - only admins can repost listings
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RepostPost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let post = super::post_operations::create_post_for_post(
        post_id,
        Some(created_by),
        None,
        None,
        None,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(PostEvent::PostCreated {
        post_id: post.id,
    })
}

async fn handle_expire_post(
    post_id: PostId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    // Authorization check - only admins can expire posts
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ExpirePost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::post_operations::expire_post(post_id, &ctx.deps().db_pool).await?;

    Ok(PostEvent::PostExpired { post_id })
}

async fn handle_archive_post(
    post_id: PostId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    // Authorization check - only admins can archive posts
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ArchivePost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::post_operations::archive_post(post_id, &ctx.deps().db_pool).await?;

    Ok(PostEvent::PostArchived { post_id })
}

async fn handle_increment_post_view(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    super::post_operations::increment_post_view(post_id, &ctx.deps().db_pool).await?;
    Ok(PostEvent::PostViewed { post_id })
}

async fn handle_increment_post_click(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    super::post_operations::increment_post_click(post_id, &ctx.deps().db_pool).await?;
    Ok(PostEvent::PostClicked { post_id })
}

async fn handle_create_posts_from_resource_link(
    _job_id: JobId,
    url: String,
    posts: Vec<ExtractedPost>,
    context: Option<String>,
    _submitter_contact: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    use crate::domains::posts::models::Post;
    use crate::domains::website::models::Website;

    // Find the organization source that was created during submission
    let organization_name = context
        .clone()
        .unwrap_or_else(|| "Submitted Resource".to_string());

    // Find the source by URL/domain (it was created in the mutation)
    let source = Website::find_by_domain(&url, &ctx.deps().db_pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Website source not found for URL: {}", url))?;

    let source_id = source.id;

    // Update last_scraped_at now that we've processed it
    Website::update_last_scraped(source_id, &ctx.deps().db_pool).await?;

    // Create each extracted listing as a user_submitted listing in pending_approval status
    let mut created_count = 0;

    for extracted_post in posts {
        // TODO: Store contact info when Listing model supports it
        let _contact_json = extracted_post
            .contact
            .and_then(|c| serde_json::to_value(c).ok());

        match Post::create(
            organization_name.clone(),
            extracted_post.title.clone(),
            extracted_post.description.clone(),
            Some(extracted_post.tldr),
            "opportunity".to_string(),
            "general".to_string(),
            Some("accepting".to_string()),
            extracted_post.urgency,
            None, // location
            "pending_approval".to_string(),
            "en".to_string(), // source_language
            Some("user_submitted".to_string()),
            None, // submitted_by_admin_id
            Some(WebsiteId::from_uuid(source_id.into_uuid())),
            Some(url.clone()),
            None, // organization_id
            &ctx.deps().db_pool,
        )
        .await
        {
            Ok(new_post) => {
                created_count += 1;
                info!(
                    post_id = %new_post.id,
                    org = %new_post.organization_name,
                    title = %new_post.title,
                    "Created listing from resource link"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    title = %extracted_post.title,
                    "Failed to create listing from resource link"
                );
            }
        }
    }

    info!(created_count = %created_count, "Created listings from resource link");

    // Return a success event (we'll use PostEntryCreated for now, but could create a new event type)
    // For simplicity, just return a generic success event
    Ok(PostEvent::PostEntryCreated {
        post_id: crate::common::PostId::new(), // Dummy ID
        organization_name: "Resource Link".to_string(),
        title: format!("{} listings created", created_count),
        submission_type: "user_submitted".to_string(),
    })
}

/// Extract domain from URL (e.g., "https://example.org/path" -> "example.org")
pub fn extract_domain(url: &str) -> Option<String> {
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
) -> Result<PostEvent> {
    use crate::common::JobId;
    use crate::domains::website::models::Website;

    // Validate URL format
    url::Url::parse(&url).context("Invalid URL format")?;

    // Generate a new job ID for tracking the scraping workflow
    let job_id = JobId::new();

    info!(
        url = %url,
        organization_name = %organization_name,
        job_id = %job_id,
        "Processing submitted resource link"
    );

    // Extract domain from submitted URL
    let domain = extract_domain(&url)
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: could not extract domain"))?;

    info!(domain = %domain, "Extracted domain from URL");

    // Find or create website (handles race conditions gracefully)
    let source = Website::find_or_create(
        url.clone(),
        None, // Public submission (no logged-in user)
        "public_user".to_string(),
        submitter_contact.clone(),
        3, // max_crawl_depth - reasonable default
        &ctx.deps().db_pool,
    )
    .await?;

    info!(
        source_id = %source.id,
        domain = %source.domain,
        status = %source.status,
        "Found or created website"
    );

    let (source_id, event_type) = (
        source.id,
        if source.status == "pending_review" {
            "created_pending_review"
        } else {
            "existing_website"
        },
    );

    info!(
        source_id = %source_id,
        job_id = %job_id,
        event_type = %event_type,
        "Website source processed successfully"
    );

    // Return appropriate event based on website status
    if event_type == "created_pending_review" {
        // Website needs approval before scraping
        Ok(PostEvent::WebsitePendingApproval {
            website_id: source_id,
            url: domain,
            submitted_url: url,
            submitter_contact,
        })
    } else {
        // Domain exists and approved - proceed with scraping
        Ok(PostEvent::WebsiteCreatedFromLink {
            source_id,
            job_id,
            url,
            organization_name,
            submitter_contact,
        })
    }
}

/// Handle DeletePost command
async fn handle_delete_post(
    post_id: PostId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    // Check authorization - only admins can delete listings
    Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
        .map_err(|e| {
            info!(
                user_id = %requested_by,
                action = "DeletePost",
                error = ?e,
                "Authorization denied"
            );
            anyhow::anyhow!("Authorization denied: {:?}", e)
        })?;

    info!(post_id = %post_id, "Deleting listing");

    // Delete the listing
    super::post_operations::delete_post(post_id, &ctx.deps().db_pool).await?;

    Ok(PostEvent::PostDeleted { post_id })
}

// ============================================================================
// Deduplication handlers
// ============================================================================

/// Handle DeduplicatePosts command - find and merge duplicate posts using embedding similarity
/// Handle deduplication of posts using LLM-based semantic analysis
///
/// This uses LLM to identify duplicate posts across ALL websites (not just one).
/// Core principle: Post identity = Organization × Service × Audience
async fn handle_deduplicate_posts(
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    use crate::domains::posts::effects::deduplication::{deduplicate_posts_llm, apply_dedup_results};
    use crate::domains::website::models::Website;

    // Authorization check - only admins can deduplicate posts
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "DeduplicatePosts".to_string(),
            reason: auth_err.to_string(),
        });
    }

    info!(
        job_id = %job_id,
        "Starting LLM-based post deduplication"
    );

    // Get all approved websites and deduplicate each
    let websites = match Website::find_approved(&ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, "Failed to fetch websites for deduplication");
            return Ok(PostEvent::PostsDeduplicated {
                job_id,
                duplicates_found: 0,
                posts_merged: 0,
                posts_deleted: 0,
            });
        }
    };

    let mut total_deleted = 0;
    let mut total_groups = 0;

    for website in &websites {
        // Run LLM deduplication for this website
        let dedup_result = match deduplicate_posts_llm(
            website.id,
            ctx.deps().ai.as_ref(),
            &ctx.deps().db_pool,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    website_id = %website.id,
                    error = %e,
                    "Failed to run LLM deduplication for website"
                );
                continue;
            }
        };

        total_groups += dedup_result.duplicate_groups.len();

        // Apply the results
        let deleted = match apply_dedup_results(
            dedup_result,
            ctx.deps().ai.as_ref(),
            &ctx.deps().db_pool,
        )
        .await
        {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    website_id = %website.id,
                    error = %e,
                    "Failed to apply deduplication results"
                );
                continue;
            }
        };

        total_deleted += deleted;

        if deleted > 0 {
            info!(
                website_id = %website.id,
                deleted = deleted,
                "Deduplicated posts for website"
            );
        }
    }

    info!(
        job_id = %job_id,
        total_groups = total_groups,
        total_deleted = total_deleted,
        "LLM deduplication complete"
    );

    Ok(PostEvent::PostsDeduplicated {
        job_id,
        duplicates_found: total_groups,
        posts_merged: total_groups,
        posts_deleted: total_deleted,
    })
}
