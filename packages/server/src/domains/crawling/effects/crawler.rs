//! CrawlerEffect - Handles multi-page website crawling workflow
//!
//! This effect watches FACT events and calls handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Cascade flow:
//!   WebsiteIngested → handle_extract_posts_from_pages → PostsExtractedFromPages
//!   WebsitePostsRegenerated → handle_extract_posts_from_pages → PostsExtractedFromPages
//!   WebsitePagesDiscovered → handle_extract_posts_from_pages → PostsExtractedFromPages
//!   PostsExtractedFromPages → handle_sync_crawled_posts → PostsSynced
//!   WebsiteCrawlNoListings (retry=false) → handle_mark_no_posts → WebsiteMarkedNoListings

use seesaw_core::effect;
use std::sync::Arc;

use crate::common::AppState;
use crate::domains::crawling::events::CrawlEvent;
use crate::kernel::ServerDeps;

use super::handlers;

/// Build the crawler effect handler.
///
/// This effect watches FACT events and calls handlers directly for cascading.
/// No *Requested events - the effect IS the cascade controller.
pub fn crawler_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>().run(|event: Arc<CrawlEvent>, ctx| async move {
        match event.as_ref() {
            // =================================================================
            // Cascade: WebsiteIngested → extract posts from ingested pages
            // =================================================================
            CrawlEvent::WebsiteIngested {
                website_id, job_id, ..
            } => handlers::handle_extract_posts_from_pages(*website_id, *job_id, &ctx).await,

            // =================================================================
            // Cascade: WebsitePostsRegenerated → extract posts from existing pages
            // =================================================================
            CrawlEvent::WebsitePostsRegenerated {
                website_id, job_id, ..
            } => handlers::handle_extract_posts_from_pages(*website_id, *job_id, &ctx).await,

            // =================================================================
            // Cascade: WebsitePagesDiscovered → extract posts from discovered pages
            // =================================================================
            CrawlEvent::WebsitePagesDiscovered {
                website_id, job_id, ..
            } => handlers::handle_extract_posts_from_pages(*website_id, *job_id, &ctx).await,

            // =================================================================
            // Cascade: PostsExtractedFromPages → sync posts to database
            // =================================================================
            CrawlEvent::PostsExtractedFromPages {
                website_id,
                job_id,
                posts,
                page_results,
            } => {
                handlers::handle_sync_crawled_posts(
                    *website_id,
                    *job_id,
                    posts.clone(),
                    page_results.clone(),
                    &ctx,
                )
                .await
            }

            // =================================================================
            // Cascade: WebsiteCrawlNoListings → mark as no posts (no retry)
            // =================================================================
            CrawlEvent::WebsiteCrawlNoListings {
                website_id, job_id, ..
            } => {
                // Retry logic is deprecated - always mark as no posts
                handlers::handle_mark_no_posts(*website_id, *job_id, &ctx).await
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            CrawlEvent::WebsiteMarkedNoListings { .. }
            | CrawlEvent::PostsSynced { .. }
            | CrawlEvent::PageSummariesRegenerated { .. }
            | CrawlEvent::PageSummaryRegenerated { .. }
            | CrawlEvent::PagePostsRegenerated { .. } => Ok(()),
        }
    })
}
