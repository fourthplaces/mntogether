//! Post composite effect - atomized event chain
//!
//! Event-driven pipeline with atomized effects - each step does ONE thing:
//!
//! ```text
//! WebsiteCreatedFromLink
//!   → scrape_effect (queued) → RETURNS ResourceLinkScraped
//!
//! ResourceLinkScraped
//!   → extract_effect (queued) → RETURNS ResourceLinkPostsExtracted
//!
//! ResourceLinkPostsExtracted
//!   → create_effect (inline) → terminal
//! ```

use anyhow::Result;
use seesaw_core::{effect, effects, EffectContext};
use tracing::info;

use crate::common::{AppState, ExtractedPost, JobId};
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

use super::ai::handle_extract_posts_from_resource_link;
use super::post::handle_create_posts_from_resource_link;
use super::scraper::handle_scrape_resource_link;

#[effects]
pub mod handlers {
    use super::*;

    #[effect(
        on = [PostEvent::WebsiteCreatedFromLink],
        extract(job_id, url, submitter_contact),
        id = "resource_link_scrape",
        retry = 2,
        timeout_secs = 60
    )]
    async fn scrape_resource_link(
        job_id: JobId,
        url: String,
        submitter_contact: Option<String>,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<PostEvent> {
        info!(job_id = %job_id, url = %url, "Starting resource link scrape (queued)");

        let event =
            handle_scrape_resource_link(job_id, url.clone(), None, submitter_contact, ctx.deps())
                .await?;

        info!(url = %url, "Scrape complete, returning ResourceLinkScraped");
        Ok(event)
    }

    #[effect(
        on = [PostEvent::ResourceLinkScraped],
        extract(job_id, url, content, context, submitter_contact),
        id = "resource_link_extract",
        retry = 2,
        timeout_secs = 60
    )]
    async fn extract_posts(
        job_id: JobId,
        url: String,
        content: String,
        context: Option<String>,
        submitter_contact: Option<String>,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<PostEvent> {
        info!(job_id = %job_id, url = %url, "Starting post extraction (queued)");

        let event = handle_extract_posts_from_resource_link(
            job_id,
            url.clone(),
            content,
            context,
            submitter_contact,
            ctx.deps(),
        )
        .await?;

        info!(url = %url, "Extraction complete, returning ResourceLinkPostsExtracted");
        Ok(event)
    }

    #[effect(
        on = [PostEvent::ResourceLinkPostsExtracted],
        extract(job_id, url, posts, context, submitter_contact),
        id = "resource_link_create"
    )]
    async fn create_posts(
        job_id: JobId,
        url: String,
        posts: Vec<ExtractedPost>,
        context: Option<String>,
        submitter_contact: Option<String>,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        info!(job_id = %job_id, url = %url, posts_count = posts.len(), "Starting post creation");

        let event = handle_create_posts_from_resource_link(
            job_id,
            url.clone(),
            posts,
            context,
            submitter_contact,
            ctx.deps(),
        )
        .await?;

        if let PostEvent::PostEntryCreated { ref title, .. } = event {
            info!(url = %url, title = %title, "Posts created from resource link");
        }
        Ok(())
    }
}
