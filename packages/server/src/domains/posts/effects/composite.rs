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
//!   → create_effect (inline) → RETURNS PostEntryCreated (terminal)
//! ```

use std::time::Duration;

use seesaw_core::effect;
use tracing::info;

use crate::common::{AppState, ExtractedPost, JobId};
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

use super::ai::handle_extract_posts_from_resource_link;
use super::post::handle_create_posts_from_resource_link;
use super::scraper::handle_scrape_resource_link;

/// Step 1: Scrape effect - handles WebsiteCreatedFromLink (queued)
/// RETURNS ResourceLinkScraped
pub fn scrape_resource_link_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<PostEvent>()
        .extract(|event| match event {
            PostEvent::WebsiteCreatedFromLink {
                job_id,
                url,
                submitter_contact,
                ..
            } => Some((*job_id, url.clone(), submitter_contact.clone())),
            _ => None,
        })
        .id("resource_link_scrape")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(60))
        .then(
            |(job_id, url, submitter_contact): (JobId, String, Option<String>),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(job_id = %job_id, url = %url, "Starting resource link scrape (queued)");

                let event = handle_scrape_resource_link(
                    job_id,
                    url.clone(),
                    None,
                    submitter_contact,
                    ctx.deps(),
                )
                .await?;

                info!(url = %url, "Scrape complete, returning ResourceLinkScraped");
                Ok(event)
            },
        )
}

/// Step 2: Extract effect - handles ResourceLinkScraped (queued)
/// RETURNS ResourceLinkPostsExtracted
pub fn extract_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<PostEvent>()
        .extract(|event| match event {
            PostEvent::ResourceLinkScraped {
                job_id,
                url,
                content,
                context,
                submitter_contact,
                ..
            } => Some((
                *job_id,
                url.clone(),
                content.clone(),
                context.clone(),
                submitter_contact.clone(),
            )),
            _ => None,
        })
        .id("resource_link_extract")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(60))
        .then(
            |(job_id, url, content, context, submitter_contact): (
                JobId,
                String,
                String,
                Option<String>,
                Option<String>,
            ),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
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
            },
        )
}

/// Step 3: Create effect - handles ResourceLinkPostsExtracted (inline — fast DB operation)
/// Terminal handler - creates posts and returns unit.
pub fn create_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<PostEvent>()
        .extract(|event| match event {
            PostEvent::ResourceLinkPostsExtracted {
                job_id,
                url,
                posts,
                context,
                submitter_contact,
                ..
            } => Some((
                *job_id,
                url.clone(),
                posts.clone(),
                context.clone(),
                submitter_contact.clone(),
            )),
            _ => None,
        })
        .id("resource_link_create")
        .then(
            |(job_id, url, posts, context, submitter_contact): (
                JobId,
                String,
                Vec<ExtractedPost>,
                Option<String>,
                Option<String>,
            ),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
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
            },
        )
}

/// Composite effect combining all three steps.
/// Each effect handles one event and returns the next, enabling automatic chaining.
pub fn post_composite_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    seesaw_core::effect::group([
        scrape_resource_link_effect(),
        extract_posts_effect(),
        create_posts_effect(),
    ])
}
