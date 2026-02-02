//! Effects (side effects) for website domain
//!
//! Effects are thin orchestrators that delegate to handler functions.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::info;

use crate::common::{MemberId, WebsiteId};
use crate::domains::website::commands::WebsiteCommand;
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website::models::Website;

// Import ServerDeps from kernel
pub use crate::kernel::ServerDeps;

/// Website Effect - Handles website approval and management commands
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct WebsiteEffect;

#[async_trait]
impl Effect<WebsiteCommand, ServerDeps> for WebsiteEffect {
    type Event = WebsiteEvent;

    async fn execute(
        &self,
        cmd: WebsiteCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<WebsiteEvent> {
        match cmd {
            WebsiteCommand::ApproveWebsite {
                website_id,
                requested_by,
            } => handle_approve_website(website_id, requested_by, &ctx).await,

            WebsiteCommand::RejectWebsite {
                website_id,
                reason,
                requested_by,
            } => handle_reject_website(website_id, reason, requested_by, &ctx).await,

            WebsiteCommand::SuspendWebsite {
                website_id,
                reason,
                requested_by,
            } => handle_suspend_website(website_id, reason, requested_by, &ctx).await,

            WebsiteCommand::UpdateCrawlSettings {
                website_id,
                max_pages_per_crawl,
                requested_by,
            } => handle_update_crawl_settings(website_id, max_pages_per_crawl, requested_by, &ctx).await,
        }
    }
}

// ============================================================================
// Handler Functions
// ============================================================================

async fn handle_approve_website(
    website_id: WebsiteId,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<WebsiteEvent> {
    info!(website_id = %website_id, requested_by = %requested_by, "Approving website");

    Website::approve(website_id, requested_by, &ctx.deps().db_pool).await?;

    Ok(WebsiteEvent::WebsiteApproved {
        website_id,
        reviewed_by: requested_by,
    })
}

async fn handle_reject_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<WebsiteEvent> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Rejecting website");

    Website::reject(website_id, requested_by, reason.clone(), &ctx.deps().db_pool).await?;

    Ok(WebsiteEvent::WebsiteRejected {
        website_id,
        reason,
        reviewed_by: requested_by,
    })
}

async fn handle_suspend_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<WebsiteEvent> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Suspending website");

    Website::suspend(website_id, requested_by, reason.clone(), &ctx.deps().db_pool).await?;

    Ok(WebsiteEvent::WebsiteSuspended {
        website_id,
        reason,
        reviewed_by: requested_by,
    })
}

async fn handle_update_crawl_settings(
    website_id: WebsiteId,
    max_pages_per_crawl: i32,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<WebsiteEvent> {
    info!(
        website_id = %website_id,
        max_pages_per_crawl = max_pages_per_crawl,
        requested_by = %requested_by,
        "Updating website crawl settings"
    );

    Website::update_max_pages_per_crawl(website_id, max_pages_per_crawl, &ctx.deps().db_pool).await?;

    Ok(WebsiteEvent::CrawlSettingsUpdated {
        website_id,
        max_pages_per_crawl,
    })
}
