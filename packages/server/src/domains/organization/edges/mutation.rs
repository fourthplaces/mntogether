use super::types::{EditNeedInput, Need, ScrapeJobResult, SubmitNeedInput};
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
/// Following seesaw pattern: create job, dispatch event, return immediately
pub async fn scrape_organization(
    ctx: &GraphQLContext,
    source_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(source_id = %source_id, "Scraping organization source");

    // Require admin access
    ctx.require_admin()?;

    // Create scrape job (pending status)
    let job = ScrapeJob::create(source_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to create scrape job: {}", e),
                juniper::Value::null(),
            )
        })?;

    // Emit request event (async workflow starts, fire-and-forget)
    ctx.bus.emit(OrganizationEvent::ScrapeSourceRequested {
        source_id,
        job_id: job.id,
    });

    // Return immediately with job_id
    Ok(ScrapeJobResult {
        job_id: job.id,
        source_id,
        status: job.status.to_string(),
    })
}

/// Submit a need from a volunteer (user-submitted, goes to pending_approval)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn submit_need(
    ctx: &GraphQLContext,
    input: SubmitNeedInput,
    volunteer_id: Option<Uuid>,
    ip_address: Option<String>,
) -> FieldResult<Need> {
    info!(
        org = %input.organization_name,
        title = %input.title,
        volunteer_id = ?volunteer_id,
        "Submitting user need"
    );

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());

    // Dispatch request event and await NeedCreated fact event
    let need_id = dispatch_request(
        OrganizationEvent::SubmitNeedRequested {
            volunteer_id,
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

    // Require admin access
    ctx.require_admin()?;

    // Dispatch request event and await NeedApproved fact event
    dispatch_request(
        OrganizationEvent::ApproveNeedRequested { need_id },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedApproved { need_id: nid } if nid == &need_id => Some(Ok(())),
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

    // Require admin access
    ctx.require_admin()?;

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());

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
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedApproved { need_id: nid } if nid == &need_id => Some(Ok(())),
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

    // Require admin access
    ctx.require_admin()?;

    // Dispatch request event and await NeedRejected fact event
    dispatch_request(
        OrganizationEvent::RejectNeedRequested { need_id, reason },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::NeedRejected { need_id: nid, .. } if nid == &need_id => {
                    Some(Ok(()))
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
