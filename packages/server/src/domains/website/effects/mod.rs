//! Website domain effect - handles cascading reactions to fact events
//!
//! Cascade flow:
//!   WebsiteApproved → auto-trigger crawl (queued)

pub mod approval;

use anyhow::Result;
use seesaw_core::{effect, effects, EffectContext};
use tracing::info;

use crate::common::{AppState, MemberId, WebsiteId};
use crate::domains::crawling::actions as crawling_actions;
use crate::domains::website::events::WebsiteEvent;
use crate::kernel::ServerDeps;

#[effects]
pub mod handlers {
    use super::*;

    #[effect(
        on = [WebsiteEvent::WebsiteApproved],
        extract(website_id, reviewed_by),
        id = "website_crawl_on_approval",
        retry = 3,
        timeout_secs = 600
    )]
    async fn crawl_on_approval(
        website_id: WebsiteId,
        reviewed_by: MemberId,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        info!(
            website_id = %website_id,
            reviewed_by = %reviewed_by,
            "Website approved, triggering auto-crawl (queued)"
        );

        crawling_actions::ingest_website(
            website_id.into_uuid(),
            reviewed_by.into_uuid(),
            true, // use_firecrawl
            true, // is_admin
            ctx.deps(),
        )
        .await?;

        Ok(()) // Terminal - crawling domain handles further events
    }
}
