use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use serde_json::Value as JsonValue;

use super::deps::ServerDeps;
use crate::common::auth::{Actor, AdminCapability, AuthError};
use crate::common::{ExtractedNeed, JobId, MemberId, NeedId, PostId};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Need Effect - Handles CreateNeed, UpdateNeedStatus, UpdateNeedAndApprove, CreatePost, GenerateNeedEmbedding commands
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct NeedEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for NeedEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::CreateNeed {
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
                handle_create_need(
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

            OrganizationCommand::UpdateNeedStatus {
                need_id,
                status,
                rejection_reason,
                requested_by,
                is_admin,
            } => {
                handle_update_need_status(
                    need_id,
                    status,
                    rejection_reason,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::UpdateNeedAndApprove {
                need_id,
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
                handle_update_need_and_approve(
                    need_id,
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

            OrganizationCommand::CreatePost {
                need_id,
                created_by,
                custom_title,
                custom_description,
                expires_in_days,
            } => {
                handle_create_post(
                    need_id,
                    created_by,
                    custom_title,
                    custom_description,
                    expires_in_days,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::GenerateNeedEmbedding { need_id } => {
                handle_generate_need_embedding(need_id, &ctx).await
            }

            OrganizationCommand::CreateCustomPost {
                need_id,
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
                    need_id,
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

            OrganizationCommand::RepostNeed {
                need_id,
                created_by,
                requested_by,
                is_admin,
            } => handle_repost_need(need_id, created_by, requested_by, is_admin, &ctx).await,

            OrganizationCommand::ExpirePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_expire_post(post_id, requested_by, is_admin, &ctx).await,

            OrganizationCommand::ArchivePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_archive_post(post_id, requested_by, is_admin, &ctx).await,

            OrganizationCommand::IncrementPostView { post_id } => {
                handle_increment_post_view(post_id, &ctx).await
            }

            OrganizationCommand::IncrementPostClick { post_id } => {
                handle_increment_post_click(post_id, &ctx).await
            }

            OrganizationCommand::CreateNeedsFromResourceLink {
                job_id,
                url,
                needs,
                context,
                submitter_contact,
            } => {
                handle_create_needs_from_resource_link(
                    job_id,
                    url,
                    needs,
                    context,
                    submitter_contact,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::CreateOrganizationSourceFromLink {
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

            _ => anyhow::bail!("NeedEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Need handlers
// ============================================================================

async fn handle_create_need(
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
) -> Result<OrganizationEvent> {
    let need = super::need_operations::create_need(
        member_id,
        organization_name.clone(),
        title,
        description,
        contact_info,
        urgency,
        location,
        ip_address,
        submission_type.clone(),
        None, // source_id
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::NeedCreated {
        need_id: need.id,
        organization_name: need.organization_name,
        title: need.title,
        submission_type,
    })
}

async fn handle_update_need_status(
    need_id: NeedId,
    status: String,
    rejection_reason: Option<String>,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can update need status
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdateNeedStatus".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let updated_status =
        super::need_operations::update_need_status(need_id, status.clone(), &ctx.deps().db_pool)
            .await?;

    if updated_status == "active" {
        Ok(OrganizationEvent::NeedApproved { need_id })
    } else if updated_status == "rejected" {
        Ok(OrganizationEvent::NeedRejected {
            need_id,
            reason: rejection_reason.unwrap_or_else(|| "No reason provided".to_string()),
        })
    } else {
        Ok(OrganizationEvent::NeedUpdated { need_id })
    }
}

async fn handle_update_need_and_approve(
    need_id: NeedId,
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
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can edit and approve needs
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdateNeedAndApprove".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::need_operations::update_and_approve_need(
        need_id,
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

    Ok(OrganizationEvent::NeedApproved { need_id })
}

async fn handle_generate_need_embedding(
    need_id: NeedId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    match super::need_operations::generate_need_embedding(
        need_id,
        ctx.deps().embedding_service.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(dimensions) => Ok(OrganizationEvent::NeedEmbeddingGenerated {
            need_id,
            dimensions,
        }),
        Err(e) => Ok(OrganizationEvent::NeedEmbeddingFailed {
            need_id,
            reason: e.to_string(),
        }),
    }
}

// ============================================================================
// Post handlers
// ============================================================================

async fn handle_create_post(
    need_id: NeedId,
    created_by: Option<MemberId>,
    custom_title: Option<String>,
    custom_description: Option<String>,
    expires_in_days: Option<i64>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    let post = super::need_operations::create_post_for_need(
        need_id,
        created_by,
        custom_title,
        custom_description,
        expires_in_days,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::PostCreated {
        post_id: post.id,
        need_id,
    })
}

async fn handle_create_custom_post(
    need_id: NeedId,
    custom_title: Option<String>,
    custom_description: Option<String>,
    custom_tldr: Option<String>,
    targeting_hints: Option<JsonValue>,
    expires_in_days: Option<i64>,
    created_by: MemberId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can create custom posts
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CreateCustomPost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let post = super::need_operations::create_custom_post(
        need_id,
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

    Ok(OrganizationEvent::PostCreated {
        post_id: post.id,
        need_id,
    })
}

async fn handle_repost_need(
    need_id: NeedId,
    created_by: MemberId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can repost needs
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RepostNeed".to_string(),
            reason: auth_err.to_string(),
        });
    }

    let post = super::need_operations::create_post_for_need(
        need_id,
        Some(created_by),
        None,
        None,
        None,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::PostCreated {
        post_id: post.id,
        need_id,
    })
}

async fn handle_expire_post(
    post_id: PostId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can expire posts
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ExpirePost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::need_operations::expire_post(post_id, &ctx.deps().db_pool).await?;

    Ok(OrganizationEvent::PostExpired { post_id })
}

async fn handle_archive_post(
    post_id: PostId,
    requested_by: MemberId,
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can archive posts
    if let Err(auth_err) = Actor::new(requested_by)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ArchivePost".to_string(),
            reason: auth_err.to_string(),
        });
    }

    super::need_operations::archive_post(post_id, &ctx.deps().db_pool).await?;

    Ok(OrganizationEvent::PostArchived { post_id })
}

async fn handle_increment_post_view(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    super::need_operations::increment_post_view(post_id, &ctx.deps().db_pool).await?;
    Ok(OrganizationEvent::PostViewed { post_id })
}

async fn handle_increment_post_click(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    super::need_operations::increment_post_click(post_id, &ctx.deps().db_pool).await?;
    Ok(OrganizationEvent::PostClicked { post_id })
}

async fn handle_create_needs_from_resource_link(
    _job_id: JobId,
    url: String,
    needs: Vec<ExtractedNeed>,
    context: Option<String>,
    _submitter_contact: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    use crate::domains::organization::models::{OrganizationNeed, OrganizationSource};
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

    // Create each extracted need as a user_submitted need in pending_approval status
    let mut created_count = 0;

    for extracted_need in needs {
        let contact_json = extracted_need
            .contact
            .and_then(|c| serde_json::to_value(c).ok());

        // Calculate content hash for deduplication
        let content_hash = {
            use sha2::{Digest, Sha256};
            let combined = format!("{}{}", extracted_need.title, extracted_need.description);
            let mut hasher = Sha256::new();
            hasher.update(combined.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        let need = OrganizationNeed {
            id: crate::common::NeedId::new(),
            organization_name: organization_name.clone(),
            title: extracted_need.title.clone(),
            description: extracted_need.description.clone(),
            description_markdown: None,
            tldr: Some(extracted_need.tldr),
            contact_info: contact_json,
            urgency: extracted_need.urgency,
            status: "pending_approval".to_string(),
            content_hash: Some(content_hash),
            location: None,
            submission_type: Some("user_submitted".to_string()),
            submitted_by_member_id: None,
            submitted_from_ip: None,
            source_id: Some(source_id),
            source_url: Some(url.clone()),
            last_seen_at: chrono::Utc::now(),
            disappeared_at: None,
            embedding: None,
            latitude: None,
            longitude: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        match need.insert(&ctx.deps().db_pool).await {
            Ok(_) => {
                created_count += 1;
                info!(
                    need_id = %need.id,
                    org = %need.organization_name,
                    title = %need.title,
                    "Created need from resource link"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    title = %extracted_need.title,
                    "Failed to create need from resource link"
                );
            }
        }
    }

    info!(created_count = %created_count, "Created needs from resource link");

    // Return a success event (we'll use NeedCreated for now, but could create a new event type)
    // For simplicity, just return a generic success event
    Ok(OrganizationEvent::NeedCreated {
        need_id: crate::common::NeedId::new(), // Dummy ID
        organization_name: "Resource Link".to_string(),
        title: format!("{} needs created", created_count),
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
) -> Result<OrganizationEvent> {
    use crate::common::{JobId, SourceId};
    use crate::domains::organization::models::OrganizationSource;
    use tracing::info;

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
    Ok(OrganizationEvent::OrganizationSourceCreatedFromLink {
        source_id,
        job_id,
        url,
        organization_name,
        submitter_contact,
    })
}
