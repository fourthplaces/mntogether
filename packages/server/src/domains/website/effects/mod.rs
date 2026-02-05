//! Website domain effect - handles cascading reactions to fact events
//!
//! Cascade flow:
//!   WebsiteApproved → auto-trigger crawl (queued)

#![allow(deprecated)] // Uses deprecated crawl_website during migration

use std::time::Duration;

use seesaw_core::effect;
use tracing::info;

use crate::common::{AppState, MemberId, WebsiteId};
use crate::domains::crawling::actions as crawling_actions;
use crate::domains::website::events::WebsiteEvent;
use crate::kernel::ServerDeps;

/// Build the website effect handler.
///
/// WebsiteApproved → auto-trigger crawl (queued, retries 3x, 10min timeout)
pub fn website_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteEvent>()
        .extract(|event| match event {
            WebsiteEvent::WebsiteApproved {
                website_id,
                reviewed_by,
            } => Some((*website_id, *reviewed_by)),
            _ => None,
        })
        .id("website_crawl_on_approval")
        .queued()
        .retry(3)
        .timeout(Duration::from_secs(600))
        .then(
            |(website_id, reviewed_by): (WebsiteId, MemberId),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(
                    website_id = %website_id,
                    reviewed_by = %reviewed_by,
                    "Website approved, triggering auto-crawl (queued)"
                );

                crawling_actions::ingest_website(
                    website_id.into_uuid(),
                    reviewed_by.into_uuid(),
                    true,  // use_firecrawl
                    true,  // is_admin
                    ctx.deps(),
                )
                .await?;

                Ok(()) // Terminal - crawling domain handles further events
            },
        )
}
