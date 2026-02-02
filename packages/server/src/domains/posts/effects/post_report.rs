use crate::common::auth::{Actor, AdminCapability};
use crate::common::entity_ids::{PostId, MemberId};
use crate::domains::chatrooms::ChatRequestState;
use crate::kernel::ServerDeps;
use crate::domains::posts::events::PostEvent;
use crate::domains::posts::models::post_report::{PostReportId, PostReportRecord};
use anyhow::{Context, Result};
use seesaw_core::EffectContext;

pub async fn handle_create_report(
    post_id: PostId,
    reported_by: Option<MemberId>,
    reporter_email: Option<String>,
    reason: String,
    category: String,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<PostEvent> {
    let report = PostReportRecord::create(
        post_id,
        reported_by,
        reporter_email,
        reason,
        category,
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to create listing report")?;

    Ok(PostEvent::PostReported {
        report_id: report.id,
        post_id,
    })
}

pub async fn handle_resolve_report(
    report_id: PostReportId,
    resolved_by: MemberId,
    resolution_notes: Option<String>,
    action_taken: String,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<PostEvent> {
    // Authorization check
    if let Err(auth_err) = Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: resolved_by,
            action: "ResolveReport".to_string(),
            reason: auth_err.to_string(),
        });
    }

    PostReportRecord::resolve(
        report_id,
        resolved_by,
        resolution_notes,
        action_taken.clone(),
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to resolve report")?;

    Ok(PostEvent::ReportResolved {
        report_id,
        action_taken,
    })
}

pub async fn handle_dismiss_report(
    report_id: PostReportId,
    resolved_by: MemberId,
    resolution_notes: Option<String>,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<PostEvent> {
    // Authorization check
    if let Err(auth_err) = Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        return Ok(PostEvent::AuthorizationDenied {
            user_id: resolved_by,
            action: "DismissReport".to_string(),
            reason: auth_err.to_string(),
        });
    }

    PostReportRecord::dismiss(
        report_id,
        resolved_by,
        resolution_notes,
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to dismiss report")?;

    Ok(PostEvent::ReportDismissed { report_id })
}
