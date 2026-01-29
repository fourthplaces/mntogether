use super::types::{EditNeedInput, Need, ScrapeJobResult, SubmitNeedInput, SubmitResourceLinkInput, SubmitResourceLinkResult};
use crate::common::{JobId, MemberId, NeedId, SourceId};
use crate::domains::matching::events::MatchingEvent;
use crate::domains::organization::data::NeedData;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::{source::OrganizationSource, OrganizationNeed, ScrapeJob};
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw::{dispatch_request, EnvelopeMatch};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Scrape an organization source (synchronous)
/// Waits for scraping to complete before returning
/// Following seesaw pattern: dispatch request event, await completion event
pub async fn scrape_organization(
    ctx: &GraphQLContext,
    source_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(source_id = %source_id, "Scraping organization source");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed IDs
    let source_id = SourceId::from_uuid(source_id);
    let job_id = JobId::new();

    // Dispatch request event and await completion (NeedsSynced or failure)
    let result = dispatch_request(
        OrganizationEvent::ScrapeSourceRequested {
            source_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                // Success - scraping workflow complete
                OrganizationEvent::NeedsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    changed_count,
                    disappeared_count,
                } if *synced_source_id == source_id && *synced_job_id == job_id => {
                    Some(Ok((
                        "completed".to_string(),
                        format!(
                            "Scraping complete! Found {} new, {} changed, {} disappeared",
                            new_count, changed_count, disappeared_count
                        ),
                    )))
                }
                // Failure events
                OrganizationEvent::ScrapeFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Scrape failed: {}", reason)))
                }
                OrganizationEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                OrganizationEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                OrganizationEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "ScrapeSource" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Scrape failed: {}", e), juniper::Value::null()))?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status,
        message: Some(message),
    })
}

/// Submit a need from a member (user-submitted, goes to pending_approval)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn submit_need(
    ctx: &GraphQLContext,
    input: SubmitNeedInput,
    member_id: Option<Uuid>,
    ip_address: Option<String>,
) -> FieldResult<Need> {
    info!(
        org = %input.organization_name,
        title = %input.title,
        member_id = ?member_id,
        "Submitting user need"
    );

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());

    // Convert to typed ID
    let member_id_typed = member_id.map(MemberId::from_uuid);

    // Dispatch request event and await NeedCreated fact event
    let need_id = dispatch_request(
        OrganizationEvent::SubmitNeedRequested {
            member_id: member_id_typed,
            organization_name: input.organization_name,
            title: input.title,
            description: input.description,
            contact_info: contact_json,
            urgency: input.urgency,
            location: input.location,
            ip_address,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedCreated {
                    need_id,
                    submission_type,
                    ..
                } if submission_type == "user_submitted" => Some(Ok(*need_id)),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to submit need: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database (read queries OK in edges)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch need", juniper::Value::null()))?;

    Ok(Need::from(need))
}

/// Approve a need (human-in-the-loop)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn approve_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<Need> {
    info!(need_id = %need_id, "Approving need (triggers matching)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let need_id = NeedId::from_uuid(need_id);

    // Dispatch request event and await NeedApproved fact event
    dispatch_request(
        OrganizationEvent::ApproveNeedRequested {
            need_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedApproved { need_id: nid } if *nid == need_id => Some(Ok(())),
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to approve need: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database (read queries OK in edges)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch need", juniper::Value::null()))?;

    Ok(Need::from(need))
}

/// Edit and approve a need (fix AI mistakes or improve user-submitted content)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn edit_and_approve_need(
    ctx: &GraphQLContext,
    need_id: Uuid,
    input: EditNeedInput,
) -> FieldResult<Need> {
    info!(need_id = %need_id, title = ?input.title, "Editing and approving need (triggers matching)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());

    // Convert to typed ID
    let need_id = NeedId::from_uuid(need_id);

    // Dispatch request event and await NeedApproved fact event
    dispatch_request(
        OrganizationEvent::EditAndApproveNeedRequested {
            need_id,
            title: input.title,
            description: input.description,
            description_markdown: input.description_markdown,
            tldr: input.tldr,
            contact_info: contact_json,
            urgency: input.urgency,
            location: input.location,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedApproved { need_id: nid } if *nid == need_id => Some(Ok(())),
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to edit and approve need: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database (read queries OK in edges)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch need", juniper::Value::null()))?;

    Ok(Need::from(need))
}

/// Reject a need (hide forever)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn reject_need(ctx: &GraphQLContext, need_id: Uuid, reason: String) -> FieldResult<bool> {
    info!(need_id = %need_id, reason = %reason, "Rejecting need");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let need_id = NeedId::from_uuid(need_id);

    // Dispatch request event and await NeedRejected fact event
    dispatch_request(
        OrganizationEvent::RejectNeedRequested {
            need_id,
            reason,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedRejected { need_id: nid, .. } if *nid == need_id => {
                    Some(Ok(()))
                }
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to reject need: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(true)
}

/// Submit a resource link (URL) from the public for scraping
/// Returns job_id immediately - user can check progress
/// Following seesaw pattern: dispatch request event, await result event with job_id
pub async fn submit_resource_link(
    ctx: &GraphQLContext,
    input: SubmitResourceLinkInput,
) -> FieldResult<SubmitResourceLinkResult> {
    info!(url = %input.url, context = ?input.context, "Submitting resource link for scraping");

    // Basic URL validation (edges can validate input)
    if !input.url.starts_with("http://") && !input.url.starts_with("https://") {
        return Err(FieldError::new(
            "Invalid URL: must start with http:// or https://",
            juniper::Value::null(),
        ));
    }

    // Dispatch request event and await OrganizationSourceCreatedFromLink event
    // This follows proper seesaw encapsulation - job_id is created in the effect
    let job_id = dispatch_request(
        OrganizationEvent::SubmitResourceLinkRequested {
            url: input.url.clone(),
            context: input.context,
            submitter_contact: input.submitter_contact,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::OrganizationSourceCreatedFromLink { job_id, .. } => {
                    Some(Ok(*job_id))
                }
                _ => None,
            })
            .result()
        },
        
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to submit resource link: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(SubmitResourceLinkResult {
        job_id: job_id.into_uuid(),
        status: "pending".to_string(),
        message: "Resource submitted successfully! We'll process it shortly.".to_string(),
    })
}

/// Delete a need (admin only)
pub async fn delete_need(
    ctx: &GraphQLContext,
    need_id: Uuid,
) -> FieldResult<bool> {
    info!(need_id = %need_id, "Deleting need");

    // Get user info and check if admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Only administrators can delete needs",
            juniper::Value::null(),
        ));
    }

    // Convert to typed ID
    let need_id = NeedId::from_uuid(need_id);

    // Delete the need
    OrganizationNeed::delete(need_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to delete need: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(true)
}

/// Add a scrape URL to an organization source (admin only)
pub async fn add_organization_scrape_url(
    ctx: &GraphQLContext,
    source_id: Uuid,
    url: String,
) -> FieldResult<bool> {
    info!(source_id = %source_id, url = %url, "Adding scrape URL");

    // Get user info and check if admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Only administrators can manage scrape URLs",
            juniper::Value::null(),
        ));
    }

    // Convert to typed ID
    let source_id = SourceId::from_uuid(source_id);

    // Add the URL
    OrganizationSource::add_scrape_url(source_id, url, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to add scrape URL: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(true)
}

/// Remove a scrape URL from an organization source (admin only)
pub async fn remove_organization_scrape_url(
    ctx: &GraphQLContext,
    source_id: Uuid,
    url: String,
) -> FieldResult<bool> {
    info!(source_id = %source_id, url = %url, "Removing scrape URL");

    // Get user info and check if admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Only administrators can manage scrape URLs",
            juniper::Value::null(),
        ));
    }

    // Convert to typed ID
    let source_id = SourceId::from_uuid(source_id);

    // Remove the URL
    OrganizationSource::remove_scrape_url(source_id, url, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to remove scrape URL: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(true)
}
