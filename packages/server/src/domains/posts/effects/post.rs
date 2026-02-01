use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use serde_json::Value as JsonValue;
use tracing::info;

use crate::kernel::ServerDeps;
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{ExtractedPost, JobId, PostId, MemberId, WebsiteId};
use tracing::warn;
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Listing Effect - Handles CreatePostEntry, UpdatePostStatus, UpdatePostAndApprove, CreatePost, GeneratePostEmbedding commands
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct PostEffect;

#[async_trait]
impl Effect<PostCommand, ServerDeps> for PostEffect {
    type Event = PostEvent;

    async fn execute(
        &self,
        cmd: PostCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<PostEvent> {
        match cmd {
            PostCommand::CreatePostEntry {
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
            }

            PostCommand::UpdatePostStatus {
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
            }

            PostCommand::UpdatePostAndApprove {
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
            }

            PostCommand::CreatePost {
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
            }

            PostCommand::GeneratePostEmbedding { post_id } => {
                handle_generate_post_embedding(post_id, &ctx).await
            }

            PostCommand::CreateCustomPost {
                post_id,
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
                    post_id,
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

            PostCommand::RepostPost {
                post_id,
                created_by,
                requested_by,
                is_admin,
            } => handle_repost_post(post_id, created_by, requested_by, is_admin, &ctx).await,

            PostCommand::ExpirePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_expire_post(post_id, requested_by, is_admin, &ctx).await,

            PostCommand::ArchivePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_archive_post(post_id, requested_by, is_admin, &ctx).await,

            PostCommand::IncrementPostView { post_id } => {
                handle_increment_post_view(post_id, &ctx).await
            }

            PostCommand::IncrementPostClick { post_id } => {
                handle_increment_post_click(post_id, &ctx).await
            }

            PostCommand::DeletePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_delete_post(post_id, requested_by, is_admin, &ctx).await,

            PostCommand::CreatePostsFromResourceLink {
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
            }

            PostCommand::CreateWebsiteFromLink {
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

            PostCommand::CreateReport {
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
            }

            PostCommand::ResolveReport {
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
            }

            PostCommand::DismissReport {
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
            }

            PostCommand::DeduplicatePosts {
                job_id,
                similarity_threshold,
                requested_by,
                is_admin,
            } => {
                handle_deduplicate_posts(job_id, similarity_threshold, requested_by, is_admin, &ctx)
                    .await
            }

            _ => anyhow::bail!("PostEffect: Unexpected command"),
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

async fn handle_generate_post_embedding(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    match super::post_operations::generate_post_embedding(
        post_id,
        ctx.deps().embedding_service.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(dimensions) => Ok(PostEvent::PostEmbeddingGenerated {
            post_id,
            dimensions,
        }),
        Err(e) => Ok(PostEvent::ListingEmbeddingFailed {
            post_id,
            reason: e.to_string(),
        }),
    }
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
    use crate::domains::scraping::models::Website;
    use tracing::info;

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
    sqlx::query("UPDATE websites SET last_scraped_at = NOW() WHERE id = $1")
        .bind(source_id.as_uuid())
        .execute(&ctx.deps().db_pool)
        .await?;

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
                tracing::warn!(
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
    use crate::domains::scraping::models::Website;
    use tracing::info;

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
async fn handle_deduplicate_posts(
    job_id: JobId,
    similarity_threshold: f32,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    use crate::domains::posts::models::Post;

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
        similarity_threshold = %similarity_threshold,
        "Starting post deduplication"
    );

    // Find all posts with embeddings
    let posts_with_embeddings = Post::find_all_with_embeddings(&ctx.deps().db_pool).await?;

    info!(
        count = posts_with_embeddings.len(),
        "Found posts with embeddings"
    );

    if posts_with_embeddings.len() < 2 {
        return Ok(PostEvent::PostsDeduplicated {
            job_id,
            duplicates_found: 0,
            posts_merged: 0,
            posts_deleted: 0,
        });
    }

    // Find duplicate groups using embedding similarity
    let duplicate_groups =
        find_duplicate_groups(&posts_with_embeddings, similarity_threshold).await;

    let duplicates_found = duplicate_groups.len();
    let mut posts_deleted = 0;

    info!(
        duplicate_groups = duplicates_found,
        "Found duplicate groups"
    );

    // Process each group - keep oldest, delete others
    for group in &duplicate_groups {
        if group.len() < 2 {
            continue;
        }

        // Sort by created_at to find oldest (keep) vs newer (delete)
        let mut sorted_group = group.clone();
        sorted_group.sort_by_key(|(_, created_at)| *created_at);

        // Keep the oldest post
        let (keeper_id, _) = sorted_group[0];
        info!(keeper_id = %keeper_id, "Keeping oldest post in duplicate group");

        // Delete the rest
        for (post_id, _) in sorted_group.iter().skip(1) {
            match Post::delete(*post_id, &ctx.deps().db_pool).await {
                Ok(_) => {
                    posts_deleted += 1;
                    info!(post_id = %post_id, "Deleted duplicate post");
                }
                Err(e) => {
                    warn!(post_id = %post_id, error = %e, "Failed to delete duplicate post");
                }
            }
        }
    }

    info!(
        duplicates_found = duplicates_found,
        posts_deleted = posts_deleted,
        "Deduplication complete"
    );

    Ok(PostEvent::PostsDeduplicated {
        job_id,
        duplicates_found,
        posts_merged: duplicates_found, // Each group is "merged" into one
        posts_deleted,
    })
}

/// Find groups of duplicate posts based on embedding cosine similarity
async fn find_duplicate_groups(
    posts: &[(PostId, Vec<f32>, chrono::DateTime<chrono::Utc>)],
    threshold: f32,
) -> Vec<Vec<(PostId, chrono::DateTime<chrono::Utc>)>> {
    use std::collections::HashSet;

    let mut processed: HashSet<PostId> = HashSet::new();
    let mut groups: Vec<Vec<(PostId, chrono::DateTime<chrono::Utc>)>> = Vec::new();

    for (i, (post_id, embedding, created_at)) in posts.iter().enumerate() {
        if processed.contains(post_id) {
            continue;
        }

        let mut group = vec![(*post_id, *created_at)];
        processed.insert(*post_id);

        // Compare with all other posts
        for (other_id, other_embedding, other_created_at) in posts.iter().skip(i + 1) {
            if processed.contains(other_id) {
                continue;
            }

            let similarity = cosine_similarity(embedding, other_embedding);
            if similarity >= threshold {
                group.push((*other_id, *other_created_at));
                processed.insert(*other_id);
            }
        }

        // Only add groups with duplicates (more than 1 post)
        if group.len() > 1 {
            groups.push(group);
        }
    }

    groups
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}
