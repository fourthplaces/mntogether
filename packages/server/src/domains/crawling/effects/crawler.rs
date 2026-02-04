//! CrawlerEffect - Handles multi-page website crawling workflow
//!
//! This effect watches FACT events and enqueues jobs for cascading.
//! NO *Requested events - GraphQL calls actions, effects enqueue jobs on facts.
//!
//! ## Job-Based Cascade Flow
//!
//! ```text
//! WebsiteIngested → ExtractPostsJob → PostsExtractedFromPages
//! WebsitePostsRegenerated → ExtractPostsJob → PostsExtractedFromPages
//! WebsitePagesDiscovered → ExtractPostsJob → PostsExtractedFromPages
//! PostsExtractedFromPages → SyncPostsJob → PostsSynced
//! WebsiteCrawlNoListings → handle_mark_no_posts → WebsiteMarkedNoListings
//! ```
//!
//! Each job runs independently and can be retried without re-running previous stages.

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
            // Cascade: WebsiteIngested → enqueue ExtractPostsJob
            // =================================================================
            CrawlEvent::WebsiteIngested {
                website_id, job_id, ..
            } => handlers::handle_enqueue_extract_posts(*website_id, *job_id, &ctx).await,

            // =================================================================
            // Cascade: WebsitePostsRegenerated → enqueue ExtractPostsJob
            // =================================================================
            CrawlEvent::WebsitePostsRegenerated {
                website_id, job_id, ..
            } => handlers::handle_enqueue_extract_posts(*website_id, *job_id, &ctx).await,

            // =================================================================
            // Cascade: WebsitePagesDiscovered → enqueue ExtractPostsJob
            // =================================================================
            CrawlEvent::WebsitePagesDiscovered {
                website_id, job_id, ..
            } => handlers::handle_enqueue_extract_posts(*website_id, *job_id, &ctx).await,

            // =================================================================
            // Cascade: PostsExtractedFromPages → enqueue SyncPostsJob
            // =================================================================
            CrawlEvent::PostsExtractedFromPages {
                website_id,
                job_id,
                posts,
                page_results,
            } => {
                handlers::handle_enqueue_sync_posts(
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
            CrawlEvent::WebsiteMarkedNoListings { .. } | CrawlEvent::PostsSynced { .. } => Ok(()),
        }
    })
}
