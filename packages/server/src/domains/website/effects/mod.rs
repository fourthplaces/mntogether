//! Website domain effect - thin dispatcher to action functions
//!
//! The effect handles request events and dispatches to action functions.
//! All business logic lives in the action functions.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::info;

use crate::common::{MemberId, WebsiteId};
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website::models::Website;

// Import ServerDeps from kernel
pub use crate::kernel::ServerDeps;

/// Website Effect - Handles WebsiteEvent request events
///
/// This effect is a thin orchestration layer that dispatches request events to handler functions.
/// Fact events should never reach this effect (they're outputs, not inputs).
pub struct WebsiteEffect;

#[async_trait]
impl Effect<WebsiteEvent, ServerDeps> for WebsiteEffect {
    type Event = WebsiteEvent;

    async fn handle(
        &mut self,
        event: WebsiteEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<WebsiteEvent> {
        match event {
            // =================================================================
            // Request Events → Dispatch to Handlers
            // =================================================================
            WebsiteEvent::ApproveWebsiteRequested {
                website_id,
                requested_by,
            } => handle_approve_website(website_id, requested_by, &ctx).await,

            WebsiteEvent::RejectWebsiteRequested {
                website_id,
                reason,
                requested_by,
            } => handle_reject_website(website_id, reason, requested_by, &ctx).await,

            WebsiteEvent::SuspendWebsiteRequested {
                website_id,
                reason,
                requested_by,
            } => handle_suspend_website(website_id, reason, requested_by, &ctx).await,

            WebsiteEvent::UpdateCrawlSettingsRequested {
                website_id,
                max_pages_per_crawl,
                requested_by,
            } => {
                handle_update_crawl_settings(website_id, max_pages_per_crawl, requested_by, &ctx)
                    .await
            }

            // =================================================================
            // Fact Events → Should not reach effect (return error)
            // =================================================================
            WebsiteEvent::WebsiteApproved { .. }
            | WebsiteEvent::WebsiteRejected { .. }
            | WebsiteEvent::WebsiteSuspended { .. }
            | WebsiteEvent::CrawlSettingsUpdated { .. }
            | WebsiteEvent::AuthorizationDenied { .. } => {
                anyhow::bail!(
                    "Fact events should not be dispatched to effects. \
                     They are outputs from effects, not inputs."
                )
            }
        }
    }
}

// ============================================================================
// Handler Functions (Business Logic)
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

    Website::update_max_pages_per_crawl(website_id, max_pages_per_crawl, &ctx.deps().db_pool)
        .await?;

    Ok(WebsiteEvent::CrawlSettingsUpdated {
        website_id,
        max_pages_per_crawl,
    })
}
