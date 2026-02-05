//! CrawlerEffect - Event handlers for crawling domain
//!
//! Note: Job chaining is now handled by job handlers in job_handlers.rs.
//! These effects are kept for any event-driven behavior that isn't job-related.
//!
//! Job pipeline (handled by JobRunner + job_handlers):
//! ```text
//! CrawlWebsiteJob    → ingest_website()         → enqueue ExtractPostsJob
//! ExtractPostsJob    → extract_posts_for_domain() → enqueue SyncPostsJob
//! SyncPostsJob       → sync_and_deduplicate_posts() → terminal
//! RegeneratePostsJob → regenerate_posts()       → enqueue ExtractPostsJob
//! ```

use seesaw_core::effect::EffectContext;
use seesaw_core::on;
use tracing::info;

use crate::common::AppState;
use crate::domains::crawling::events::CrawlEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Mark no listings effect - handles WebsiteCrawlNoListings.
/// Terminal handler - logs when a website has no listings.
pub fn mark_no_listings_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    let mut effect = on! {
        CrawlEvent::WebsiteCrawlNoListings { website_id, job_id, .. } => |ctx: EffectContext<AppState, ServerDeps>| async move {
            let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;

            let total_attempts = website.crawl_attempt_count.unwrap_or(0);
            info!(
                website_id = %website_id,
                job_id = %job_id,
                total_attempts = total_attempts,
                "Website marked as having no listings"
            );
            Ok(())
        },
    };
    effect.id = "mark_no_listings".to_string();
    effect
}

// Note: extract_posts_effect and enqueue_sync_effect are removed.
// Job chaining is now handled by job handlers:
// - CrawlWebsite handler enqueues ExtractPosts
// - ExtractPosts handler enqueues SyncPosts
