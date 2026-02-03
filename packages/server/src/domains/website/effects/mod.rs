//! Website domain effect - handles cascading reactions to fact events
//!
//! Following the direct-call pattern:
//!   GraphQL → Action → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Cascade flow:
//!   WebsiteApproved → crawl_website action → WebsiteCrawled → ...

#![allow(deprecated)] // Uses deprecated crawl_website during migration

use seesaw_core::effect;
use std::sync::Arc;
use tracing::info;

use crate::common::AppState;
use crate::domains::crawling::actions as crawling_actions;
use crate::domains::website::events::WebsiteEvent;
use crate::kernel::ServerDeps;

/// Build the website effect handler.
///
/// Cascade flow:
///   WebsiteApproved → auto-trigger crawl
pub fn website_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteEvent>().run(|event: Arc<WebsiteEvent>, ctx| async move {
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

                // Trigger crawl - approver must be admin
                let _ = crawling_actions::crawl_website(
                    website_id.into_uuid(),
                    reviewed_by.into_uuid(),
                    true, // is_admin (approver must be admin)
                    &ctx,
                )
                .await;

                Ok(())
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            WebsiteEvent::WebsiteRejected { .. }
            | WebsiteEvent::WebsiteSuspended { .. }
            | WebsiteEvent::CrawlSettingsUpdated { .. }
            | WebsiteEvent::AuthorizationDenied { .. } => Ok(()),
        }
    })
}
