//! Website domain effect - handles cascading reactions to fact events
//!
//! Effects use `.then()` and return `Ok(Event)` to chain or `Ok(())` for terminal.
//!
//! Cascade flow:
//!   WebsiteApproved → auto-trigger crawl (terminal for website domain)

#![allow(deprecated)] // Uses deprecated crawl_website during migration

use seesaw_core::{effect, EffectContext};
use tracing::info;

use crate::common::AppState;
use crate::domains::crawling::actions as crawling_actions;
use crate::domains::website::events::WebsiteEvent;
use crate::kernel::ServerDeps;

/// Build the website effect handler.
///
/// Cascade flow:
///   WebsiteApproved → auto-trigger crawl (terminal)
/// Errors propagate to global on_error() handler.
pub fn website_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteEvent>().id("website_crawl_on_approval").then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: WebsiteApproved → auto-crawl the website
                // =================================================================
                WebsiteEvent::WebsiteApproved {
                    website_id,
                    reviewed_by,
                } => {
                    info!(
                        website_id = %website_id,
                        reviewed_by = %reviewed_by,
                        "Website approved, triggering auto-crawl"
                    );

                    // Trigger crawl via ingest_website (use Firecrawl for better JS rendering)
                    crawling_actions::ingest_website(
                        website_id.into_uuid(),
                        reviewed_by.into_uuid(),
                        true,  // use_firecrawl
                        true,  // is_admin (website approval is admin-only)
                        ctx.deps(),
                    )
                    .await?;

                    Ok(()) // Terminal - crawling domain handles further events
                }

                // =================================================================
                // Terminal events - no cascade needed
                // =================================================================
                WebsiteEvent::WebsiteRejected { .. }
                | WebsiteEvent::WebsiteSuspended { .. }
                | WebsiteEvent::CrawlSettingsUpdated { .. }
                | WebsiteEvent::AuthorizationDenied { .. } => Ok(()),
            }
        },
    )
}
