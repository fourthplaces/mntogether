//! CrawlerEffect - Handles multi-page website crawling workflow
//!
//! This effect watches FACT events and enqueues jobs for cascading.
//! NO *Requested events - GraphQL calls actions, effects enqueue jobs on facts.
//!
//! ## Job-Based Cascade Flow
//!
//! ```text
//! WebsiteIngested → ExtractPostsJob → SyncPostsJob (chained internally)
//! WebsitePostsRegenerated → ExtractPostsJob → SyncPostsJob (chained internally)
//! WebsitePagesDiscovered → ExtractPostsJob → SyncPostsJob (chained internally)
//! WebsiteCrawlNoListings → mark as no posts
//! ```
//!
//! Jobs are chained internally by handlers rather than via event emission.

use seesaw_core::{effect, EffectContext};
use tracing::{error, info};

use crate::common::AppState;
use crate::domains::crawling::events::CrawlEvent;
use crate::kernel::ServerDeps;

use super::handlers;

/// Build the crawler effect handler.
///
/// This effect watches FACT events and calls handlers directly for cascading.
/// Handlers chain internally (extract → sync) rather than emitting events.
pub fn crawler_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: WebsiteIngested → extract posts → sync posts (chained)
                // =================================================================
                CrawlEvent::WebsiteIngested {
                    website_id, job_id, ..
                } => {
                    match handlers::handle_enqueue_extract_posts(*website_id, *job_id, ctx.deps())
                        .await
                    {
                        Ok((posts, page_results)) => {
                            info!(
                                website_id = %website_id,
                                posts_count = posts.len(),
                                "Extraction complete, chaining to sync"
                            );
                            // Chain to sync
                            if let Err(e) = handlers::handle_enqueue_sync_posts(
                                *website_id,
                                *job_id,
                                posts,
                                page_results,
                                ctx.deps(),
                            )
                            .await
                            {
                                error!(error = %e, "Sync posts failed");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Extract posts failed");
                        }
                    }
                    Ok(()) // Terminal
                }

                // =================================================================
                // Cascade: WebsitePostsRegenerated → extract → sync (chained)
                // =================================================================
                CrawlEvent::WebsitePostsRegenerated {
                    website_id, job_id, ..
                } => {
                    match handlers::handle_enqueue_extract_posts(*website_id, *job_id, ctx.deps())
                        .await
                    {
                        Ok((posts, page_results)) => {
                            info!(
                                website_id = %website_id,
                                posts_count = posts.len(),
                                "Regeneration extraction complete, chaining to sync"
                            );
                            if let Err(e) = handlers::handle_enqueue_sync_posts(
                                *website_id,
                                *job_id,
                                posts,
                                page_results,
                                ctx.deps(),
                            )
                            .await
                            {
                                error!(error = %e, "Sync posts failed");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Extract posts failed");
                        }
                    }
                    Ok(()) // Terminal
                }

                // =================================================================
                // Cascade: WebsitePagesDiscovered → extract → sync (chained)
                // =================================================================
                CrawlEvent::WebsitePagesDiscovered {
                    website_id, job_id, ..
                } => {
                    match handlers::handle_enqueue_extract_posts(*website_id, *job_id, ctx.deps())
                        .await
                    {
                        Ok((posts, page_results)) => {
                            info!(
                                website_id = %website_id,
                                posts_count = posts.len(),
                                "Discovery extraction complete, chaining to sync"
                            );
                            if let Err(e) = handlers::handle_enqueue_sync_posts(
                                *website_id,
                                *job_id,
                                posts,
                                page_results,
                                ctx.deps(),
                            )
                            .await
                            {
                                error!(error = %e, "Sync posts failed");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Extract posts failed");
                        }
                    }
                    Ok(()) // Terminal
                }

                // =================================================================
                // PostsExtractedFromPages - handled internally above, but match for direct emission
                // =================================================================
                CrawlEvent::PostsExtractedFromPages {
                    website_id,
                    job_id,
                    posts,
                    page_results,
                } => {
                    info!(
                        website_id = %website_id,
                        posts_count = posts.len(),
                        "PostsExtractedFromPages received, syncing"
                    );
                    if let Err(e) = handlers::handle_enqueue_sync_posts(
                        *website_id,
                        *job_id,
                        posts.clone(),
                        page_results.clone(),
                        ctx.deps(),
                    )
                    .await
                    {
                        error!(error = %e, "Sync posts failed");
                    }
                    Ok(()) // Terminal
                }

                // =================================================================
                // Cascade: WebsiteCrawlNoListings → mark as no posts
                // =================================================================
                CrawlEvent::WebsiteCrawlNoListings {
                    website_id, job_id, ..
                } => {
                    match handlers::handle_mark_no_posts(*website_id, *job_id, ctx.deps()).await {
                        Ok(total_attempts) => {
                            info!(
                                website_id = %website_id,
                                total_attempts = total_attempts,
                                "Website marked as having no listings"
                            );
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to mark website as no listings");
                        }
                    }
                    Ok(()) // Terminal
                }

                // =================================================================
                // Terminal events - no cascade needed
                // =================================================================
                CrawlEvent::WebsiteMarkedNoListings { .. } | CrawlEvent::PostsSynced { .. } => {
                    Ok(())
                }
            }
        },
    )
}
