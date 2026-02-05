//! Crawling pipeline effects - queued event-driven workers
//!
//! Replaces the custom job system (CrawlWebsiteJob, ExtractPostsJob, etc.)
//! with seesaw queued effects:
//!
//! ```text
//! CrawlWebsiteEnqueued        → crawl_website_effect (queued) → RETURNS PostsExtractionEnqueued
//! PostsExtractionEnqueued     → extract_posts_effect (queued) → RETURNS PostsSyncEnqueued
//! PostsSyncEnqueued           → sync_posts_effect (queued)    → terminal
//! PostsRegenerationEnqueued   → regenerate_posts_effect (queued) → RETURNS PostsExtractionEnqueued
//! SinglePostRegenerationEnqueued → regenerate_single_post_effect (queued) → terminal
//! ```

use std::time::Duration;

use anyhow::anyhow;
use seesaw_core::effect;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, ExtractedPost, WebsiteId};
use crate::domains::crawling::actions::{
    ingest_website, regenerate_posts, regenerate_single_post, sync_and_deduplicate_posts,
};
use crate::domains::crawling::actions::post_extraction::extract_posts_for_domain;
use crate::domains::crawling::events::CrawlEvent;
use crate::domains::posts::actions::llm_sync::llm_sync_posts;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Crawl website effect - replaces CrawlWebsiteJob
///
/// CrawlWebsiteEnqueued → ingest_website() → RETURNS PostsExtractionEnqueued
pub fn crawl_website_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>()
        .extract(|event| match event {
            CrawlEvent::CrawlWebsiteEnqueued {
                website_id,
                visitor_id,
                use_firecrawl,
            } => Some((*website_id, *visitor_id, *use_firecrawl)),
            _ => None,
        })
        .id("crawl_website")
        .queued()
        .retry(3)
        .timeout(Duration::from_secs(600))
        .then(
            |(website_id, visitor_id, use_firecrawl): (Uuid, Uuid, bool),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(
                    website_id = %website_id,
                    use_firecrawl = use_firecrawl,
                    "Crawling website (queued effect)"
                );

                ingest_website(website_id, visitor_id, use_firecrawl, true, ctx.deps()).await?;

                info!(website_id = %website_id, "Crawl complete, returning extraction enqueue");

                Ok(CrawlEvent::PostsExtractionEnqueued { website_id })
            },
        )
}

/// Extract posts effect - replaces ExtractPostsJob
///
/// PostsExtractionEnqueued → extract_posts_for_domain() → RETURNS PostsSyncEnqueued
pub fn extract_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>()
        .extract(|event| match event {
            CrawlEvent::PostsExtractionEnqueued { website_id } => Some(*website_id),
            _ => None,
        })
        .id("extract_posts")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(120))
        .then(
            |website_id: Uuid,
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(website_id = %website_id, "Extracting posts (queued effect)");

                let website_id_typed = WebsiteId::from_uuid(website_id);
                let website = Website::find_by_id(website_id_typed, &ctx.deps().db_pool).await?;

                let extraction = ctx
                    .deps()
                    .extraction
                    .as_ref()
                    .ok_or_else(|| anyhow!("Extraction service not available"))?;

                let result =
                    extract_posts_for_domain(&website.domain, extraction.as_ref(), ctx.deps())
                        .await?;

                let posts_count = result.posts.len();
                info!(
                    website_id = %website_id,
                    posts_count = posts_count,
                    "Extraction complete, returning sync enqueue"
                );

                // Always return the sync event; sync effect handles empty posts gracefully
                Ok(CrawlEvent::PostsSyncEnqueued {
                    website_id: website_id_typed,
                    posts: result.posts,
                    use_llm_sync: false,
                })
            },
        )
}

/// Sync posts effect - replaces SyncPostsJob
///
/// PostsSyncEnqueued → sync_and_deduplicate_posts() (terminal)
pub fn sync_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>()
        .extract(|event| match event {
            CrawlEvent::PostsSyncEnqueued {
                website_id,
                posts,
                use_llm_sync,
            } => Some((*website_id, posts.clone(), *use_llm_sync)),
            _ => None,
        })
        .id("sync_posts")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(120))
        .then(
            |(website_id, posts, use_llm_sync): (WebsiteId, Vec<ExtractedPost>, bool),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                let posts_count = posts.len();

                if posts_count == 0 {
                    info!(website_id = %website_id, "No posts to sync, skipping");
                    return Ok(());
                }

                info!(
                    website_id = %website_id,
                    posts_count = posts_count,
                    use_llm_sync = use_llm_sync,
                    "Syncing posts (queued effect)"
                );

                if use_llm_sync {
                    let result = llm_sync_posts(
                        website_id,
                        posts,
                        ctx.deps().ai.as_ref(),
                        &ctx.deps().db_pool,
                    )
                    .await?;

                    info!(
                        website_id = %website_id,
                        inserted = result.inserted,
                        updated = result.updated,
                        deleted = result.deleted,
                        merged = result.merged,
                        "LLM sync completed"
                    );
                } else {
                    let result =
                        sync_and_deduplicate_posts(website_id, posts, ctx.deps()).await?;

                    info!(
                        website_id = %website_id,
                        inserted = result.sync_result.inserted,
                        updated = result.sync_result.updated,
                        deleted = result.sync_result.deleted,
                        merged = result.sync_result.merged,
                        "Simple sync completed"
                    );
                }

                Ok(())
            },
        )
}

/// Regenerate posts effect - replaces RegeneratePostsJob
///
/// PostsRegenerationEnqueued → regenerate_posts() → RETURNS PostsExtractionEnqueued
pub fn regenerate_posts_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>()
        .extract(|event| match event {
            CrawlEvent::PostsRegenerationEnqueued {
                website_id,
                visitor_id,
            } => Some((*website_id, *visitor_id)),
            _ => None,
        })
        .id("regenerate_posts")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(300))
        .then(
            |(website_id, visitor_id): (Uuid, Uuid),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(website_id = %website_id, "Regenerating posts (queued effect)");

                regenerate_posts(website_id, visitor_id, true, ctx.deps()).await?;

                info!(website_id = %website_id, "Regeneration complete, returning extraction enqueue");

                Ok(CrawlEvent::PostsExtractionEnqueued { website_id })
            },
        )
}

/// Regenerate single post effect - replaces RegenerateSinglePostJob
///
/// SinglePostRegenerationEnqueued → regenerate_single_post() (terminal)
pub fn regenerate_single_post_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>()
        .extract(|event| match event {
            CrawlEvent::SinglePostRegenerationEnqueued { post_id } => Some(*post_id),
            _ => None,
        })
        .id("regenerate_single_post")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(60))
        .then(
            |post_id: Uuid,
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(post_id = %post_id, "Regenerating single post (queued effect)");

                regenerate_single_post(post_id, ctx.deps()).await?;

                info!(post_id = %post_id, "Single post regeneration complete");
                Ok(())
            },
        )
}

/// Composite effect grouping all crawling pipeline effects.
pub fn crawling_pipeline_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    seesaw_core::effect::group([
        crawl_website_effect(),
        extract_posts_effect(),
        sync_posts_effect(),
        regenerate_posts_effect(),
        regenerate_single_post_effect(),
    ])
}
