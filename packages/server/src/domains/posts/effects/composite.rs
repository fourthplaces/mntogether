//! Post composite effect - atomized event chain
//!
//! Event-driven pipeline with atomized effects - each step does ONE thing:
//!
//! ```text
//! WebsiteCreatedFromLink
//!   → scrape_effect → RETURNS ResourceLinkScraped
//!
//! ResourceLinkScraped
//!   → extract_effect → RETURNS ResourceLinkPostsExtracted
//!
//! ResourceLinkPostsExtracted
//!   → create_effect → RETURNS PostEntryCreated (terminal)
//! ```

use seesaw_core::effect::EffectContext;
use seesaw_core::on;
use tracing::info;

use crate::common::AppState;
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

use super::ai::handle_extract_posts_from_resource_link;
use super::post::handle_create_posts_from_resource_link;
use super::scraper::handle_scrape_resource_link;

/// Step 1: Scrape effect - handles WebsiteCreatedFromLink
/// RETURNS ResourceLinkScraped
pub fn scrape_resource_link_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    on! {
        PostEvent::WebsiteCreatedFromLink { job_id, url, submitter_contact, .. } => |ctx: EffectContext<AppState, ServerDeps>| async move {
            info!(job_id = %job_id, url = %url, "Starting resource link scrape");

            let event = handle_scrape_resource_link(
                job_id,
                url.clone(),
                None,
                submitter_contact.clone(),
                ctx.deps(),
            ).await?;

            info!(url = %url, "Scrape complete, returning ResourceLinkScraped");
            Ok(event)
        },
    }
}

/// Step 2: Extract effect - handles ResourceLinkScraped
/// RETURNS ResourceLinkPostsExtracted
pub fn extract_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    on! {
        PostEvent::ResourceLinkScraped { job_id, url, content, context, submitter_contact, .. } => |ctx: EffectContext<AppState, ServerDeps>| async move {
            info!(job_id = %job_id, url = %url, "Starting post extraction");

            let event = handle_extract_posts_from_resource_link(
                job_id,
                url.clone(),
                content.clone(),
                context.clone(),
                submitter_contact.clone(),
                ctx.deps(),
            ).await?;

            info!(url = %url, "Extraction complete, returning ResourceLinkPostsExtracted");
            Ok(event)
        },
    }
}

/// Step 3: Create effect - handles ResourceLinkPostsExtracted
/// Terminal handler - creates posts and returns unit.
pub fn create_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    on! {
        PostEvent::ResourceLinkPostsExtracted { job_id, url, posts, context, submitter_contact, .. } => |ctx: EffectContext<AppState, ServerDeps>| async move {
            info!(job_id = %job_id, url = %url, posts_count = posts.len(), "Starting post creation");

            let event = handle_create_posts_from_resource_link(
                job_id,
                url.clone(),
                posts.clone(),
                context.clone(),
                submitter_contact.clone(),
                ctx.deps(),
            ).await?;

            if let PostEvent::PostEntryCreated { ref title, .. } = event {
                info!(url = %url, title = %title, "Posts created from resource link");
            }
            Ok(())
        },
    }
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
