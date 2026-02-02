//! Post report actions - entry-point functions for report operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return final models.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{AppState, MemberId, PostId};
use crate::domains::posts::events::PostEvent;
use crate::domains::posts::models::post_report::{PostReportId, PostReportRecord};
use crate::kernel::ServerDeps;

/// Report a post for moderation (public - no auth required)
/// Returns the created report.
pub async fn report_post(
    post_id: Uuid,
    reported_by: Option<Uuid>,
    reporter_email: Option<String>,
    reason: String,
    category: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<PostReportRecord> {
    let post_id = PostId::from_uuid(post_id);
    let reported_by = reported_by.map(MemberId::from_uuid);

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

    ctx.emit(PostEvent::PostReported {
        report_id: report.id,
        post_id,
    });

    Ok(report)
}

/// Resolve a report (admin only)
/// Returns true on success.
pub async fn resolve_report(
    report_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    resolution_notes: Option<String>,
    action_taken: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let report_id = PostReportId::from_uuid(report_id);
    let resolved_by = MemberId::from_uuid(member_id);

    if let Err(auth_err) = Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: resolved_by,
            action: "ResolveReport".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
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

    ctx.emit(PostEvent::ReportResolved {
        report_id,
        action_taken,
    });

    Ok(true)
}

/// Dismiss a report (admin only)
/// Returns true on success.
pub async fn dismiss_report(
    report_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    resolution_notes: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let report_id = PostReportId::from_uuid(report_id);
    let resolved_by = MemberId::from_uuid(member_id);

    if let Err(auth_err) = Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: resolved_by,
            action: "DismissReport".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    PostReportRecord::dismiss(report_id, resolved_by, resolution_notes, &ctx.deps().db_pool)
        .await
        .context("Failed to dismiss report")?;

    ctx.emit(PostEvent::ReportDismissed { report_id });

    Ok(true)
}
