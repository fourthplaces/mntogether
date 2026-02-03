//! CrawlerEffect - Handles multi-page website crawling workflow
//!
//! This effect watches FACT events and calls handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Cascade flow:
//!   WebsiteCrawled → handle_extract_from_pages → PostsExtractedFromPages
//!   PostsExtractedFromPages → handle_sync_crawled_posts → PostsSynced
//!   WebsiteCrawlNoListings (retry=true) → handle_retry_crawl
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
            // Cascade: WebsiteCrawled → extract posts from pages
            // =================================================================
            CrawlEvent::WebsiteCrawled {
                website_id,
                job_id,
                pages,
            } => {
                handlers::handle_extract_from_pages(*website_id, *job_id, pages.clone(), &ctx).await
            }

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
            // Cascade: WebsiteCrawlNoListings → retry or mark as no posts
            // =================================================================
            CrawlEvent::WebsiteCrawlNoListings {
                website_id,
                job_id,
                should_retry,
                ..
            } => {
                if *should_retry {
                    handlers::handle_retry_crawl(*website_id, *job_id, &ctx).await
                } else {
                    handlers::handle_mark_no_posts(*website_id, *job_id, &ctx).await
                }
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            CrawlEvent::WebsiteMarkedNoListings { .. }
            | CrawlEvent::WebsiteCrawlFailed { .. }
            | CrawlEvent::PostsSynced { .. }
            | CrawlEvent::PageSummariesRegenerated { .. }
            | CrawlEvent::PageSummaryRegenerated { .. }
            | CrawlEvent::PagePostsRegenerated { .. }
            | CrawlEvent::AuthorizationDenied { .. } => Ok(()),
        }
    })
}
