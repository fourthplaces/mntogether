use crate::common::auth::{Actor, AdminCapability};
use crate::common::entity_ids::{ListingId, MemberId};
use crate::domains::listings::effects::deps::ServerDeps;
use crate::domains::listings::events::ListingEvent;
use crate::domains::listings::models::listing_report::{ListingReportId, ListingReportRecord};
use anyhow::{Context, Result};
use seesaw_core::EffectContext;

pub async fn handle_create_report(
    listing_id: ListingId,
    reported_by: Option<MemberId>,
    reporter_email: Option<String>,
    reason: String,
    category: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    let report = ListingReportRecord::create(
        listing_id,
        reported_by,
        reporter_email,
        reason,
        category,
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to create listing report")?;

    Ok(ListingEvent::ListingReported {
        report_id: report.id,
        listing_id,
    })
}

pub async fn handle_resolve_report(
    report_id: ListingReportId,
    resolved_by: MemberId,
    resolution_notes: Option<String>,
    action_taken: String,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check
    if let Err(auth_err) = Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: resolved_by,
            action: "ResolveReport".to_string(),
            reason: auth_err.to_string(),
        });
    }

    ListingReportRecord::resolve(
        report_id,
        resolved_by,
        resolution_notes,
        action_taken.clone(),
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to resolve report")?;

    Ok(ListingEvent::ReportResolved {
        report_id,
        action_taken,
    })
}

pub async fn handle_dismiss_report(
    report_id: ListingReportId,
    resolved_by: MemberId,
    resolution_notes: Option<String>,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // Authorization check
    if let Err(auth_err) = Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: resolved_by,
            action: "DismissReport".to_string(),
            reason: auth_err.to_string(),
        });
    }

    ListingReportRecord::dismiss(
        report_id,
        resolved_by,
        resolution_notes,
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to dismiss report")?;

    Ok(ListingEvent::ReportDismissed { report_id })
}
