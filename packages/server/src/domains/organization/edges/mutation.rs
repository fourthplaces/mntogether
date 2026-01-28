use super::types::{EditNeedInput, Need, ScrapeJobResult, SubmitNeedInput};
use crate::common::{JobId, MemberId, NeedId, SourceId};
use crate::domains::matching::events::MatchingEvent;
use crate::domains::organization::data::NeedData;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::{OrganizationNeed, ScrapeJob};
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw::{dispatch_request, EnvelopeMatch};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Scrape an organization source (async)
/// Returns job_id immediately - admin polls for progress
/// Following seesaw pattern: dispatch request event, machine creates command, effect creates job
pub async fn scrape_organization(
    ctx: &GraphQLContext,
    source_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(source_id = %source_id, "Scraping organization source");

    // Get user info (authorization will be checked in effect)
    let user = ctx.auth_user.as_ref().ok_or_else(|| {
        FieldError::new("Authentication required", juniper::Value::null())
    })?;

    // Convert to typed IDs
    let source_id = SourceId::from_uuid(source_id);
    let job_id = JobId::new();

    // Emit request event (async workflow starts)
    // Machine will decide on ScrapeSource command
    // Effect will check authorization and create job
    ctx.bus.emit(OrganizationEvent::ScrapeSourceRequested {
        source_id,
        job_id,
        requested_by: user.member_id,
        is_admin: user.is_admin,
    });

    // Return immediately with job_id (job will be created in effect)
    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status: "pending".to_string(),
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
    let user = ctx.auth_user.as_ref().ok_or_else(|| {
        FieldError::new("Authentication required", juniper::Value::null())
    })?;

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
    let user = ctx.auth_user.as_ref().ok_or_else(|| {
        FieldError::new("Authentication required", juniper::Value::null())
    })?;

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
    let user = ctx.auth_user.as_ref().ok_or_else(|| {
        FieldError::new("Authentication required", juniper::Value::null())
    })?;

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
                OrganizationEvent::NeedRejected { need_id: nid, .. } if nid == need_id => {
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
