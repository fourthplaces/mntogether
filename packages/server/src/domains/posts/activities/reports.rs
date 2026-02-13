//! Post report actions - entry-point functions for report operations
//!
//! These are called from Restate virtual objects.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return plain data.

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{MemberId, PostId};
use crate::domains::posts::models::post_report::{PostReportId, PostReportRecord};
use crate::kernel::ServerDeps;

/// Result of reporting a post
pub struct ReportCreated {
    pub report_id: PostReportId,
    pub post_id: PostId,
}

/// Report a post for moderation (public - no auth required)
/// Returns the report and post IDs.
pub async fn report_post(
    post_id: Uuid,
    reported_by: Option<Uuid>,
    reporter_email: Option<String>,
    reason: String,
    category: String,
    deps: &ServerDeps,
) -> Result<ReportCreated> {
    let post_id = PostId::from_uuid(post_id);
    let reported_by = reported_by.map(MemberId::from_uuid);

    let report = PostReportRecord::create(
        post_id,
        reported_by,
        reporter_email,
        reason,
        category,
        &deps.db_pool,
    )
    .await
    .context("Failed to create listing report")?;

    Ok(ReportCreated {
        report_id: report.id,
        post_id,
    })
}

/// Resolve a report (admin only)
pub async fn resolve_report(
    report_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    resolution_notes: Option<String>,
    action_taken: String,
    deps: &ServerDeps,
) -> Result<()> {
    let report_id = PostReportId::from_uuid(report_id);
    let resolved_by = MemberId::from_uuid(member_id);

    Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    PostReportRecord::resolve(
        report_id,
        resolved_by,
        resolution_notes,
        action_taken,
        &deps.db_pool,
    )
    .await
    .context("Failed to resolve report")?;

    Ok(())
}

/// Dismiss a report (admin only)
pub async fn dismiss_report(
    report_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    resolution_notes: Option<String>,
    deps: &ServerDeps,
) -> Result<()> {
    let report_id = PostReportId::from_uuid(report_id);
    let resolved_by = MemberId::from_uuid(member_id);

    Actor::new(resolved_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    PostReportRecord::dismiss(report_id, resolved_by, resolution_notes, &deps.db_pool)
        .await
        .context("Failed to dismiss report")?;

    Ok(())
}
