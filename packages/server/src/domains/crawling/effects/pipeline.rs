//! Crawling pipeline effects - queued event-driven workers with fan-out/join
//!
//! Pipeline:
//! ```text
//! CrawlWebsiteEnqueued        → crawl_website (queued)        → PostsExtractionEnqueued
//! PostsExtractionEnqueued     → extract_narratives (queued)    → Batch([PostInvestigationEnqueued...])
//! PostInvestigationEnqueued   → investigate_post (queued)      → PostInvestigated [parallel per post]
//! PostInvestigated            → join_investigations (join)     → PostsSyncEnqueued
//! PostsSyncEnqueued           → sync_posts (queued)            → terminal
//! PostsRegenerationEnqueued   → regenerate_posts (queued)      → PostsExtractionEnqueued
//! SinglePostRegenerationEnqueued → regenerate_single_post      → terminal
//! WebsiteCrawlNoListings      → mark_no_listings              → terminal
//! ```

use anyhow::{anyhow, Result};
use seesaw_core::{effect, effects, EffectContext, Emit};
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{AppState, ExtractedPost, WebsiteId};
use crate::domains::crawling::activities::post_extraction;
use crate::domains::crawling::activities::{ingest_website, regenerate_posts, regenerate_single_post};
use crate::domains::crawling::events::CrawlEvent;
use crate::domains::posts::activities::llm_sync::llm_sync_posts;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

#[effects]
pub mod handlers {
    use super::*;

    // =========================================================================
    // Step 1: Crawl website
    // =========================================================================

    #[effect(
        on = [CrawlEvent::CrawlWebsiteEnqueued],
        extract(website_id, visitor_id, use_firecrawl),
        id = "crawl_website",
        retry = 3,
        timeout_secs = 600
    )]
    async fn crawl_website(
        website_id: Uuid,
        visitor_id: Uuid,
        use_firecrawl: bool,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<CrawlEvent> {
        info!(
            website_id = %website_id,
            use_firecrawl = use_firecrawl,
            "Crawling website (queued effect)"
        );

        ingest_website(website_id, visitor_id, use_firecrawl, true, ctx.deps()).await?;

        info!(website_id = %website_id, "Crawl complete, returning extraction enqueue");

        Ok(CrawlEvent::PostsExtractionEnqueued { website_id })
    }

    // =========================================================================
    // Step 2: Extract narratives → fan-out to investigations
    // =========================================================================

    #[effect(
        on = [CrawlEvent::PostsExtractionEnqueued],
        extract(website_id),
        id = "extract_narratives",
        retry = 2,
        timeout_secs = 120
    )]
    async fn extract_narratives(
        website_id: Uuid,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<Emit<CrawlEvent>> {
        info!(website_id = %website_id, "Extracting narratives (queued effect)");

        let website_id_typed = WebsiteId::from_uuid(website_id);
        let website = Website::find_by_id(website_id_typed, &ctx.deps().db_pool).await?;
        let extraction = ctx
            .deps()
            .extraction
            .as_ref()
            .ok_or_else(|| anyhow!("Extraction service not available"))?;

        let (narratives, _page_urls) =
            post_extraction::extract_narratives_for_domain(&website.domain, extraction.as_ref())
                .await?;

        if narratives.is_empty() {
            info!(website_id = %website_id, "No narratives found, syncing empty");
            return Ok(Emit::One(CrawlEvent::PostsSyncEnqueued {
                website_id: website_id_typed,
                posts: vec![],
            }));
        }

        info!(
            website_id = %website_id,
            narratives_count = narratives.len(),
            "Narratives extracted, fanning out to investigations"
        );

        Ok(Emit::Batch(
            narratives
                .into_iter()
                .map(|n| CrawlEvent::PostInvestigationEnqueued {
                    website_id: website_id_typed,
                    title: n.title,
                    tldr: n.tldr,
                    description: n.description,
                    source_url: n.source_url,
                })
                .collect(),
        ))
    }

    // =========================================================================
    // Step 3: Investigate individual post (parallel per narrative)
    // =========================================================================

    #[effect(
        on = [CrawlEvent::PostInvestigationEnqueued],
        extract(website_id, title, tldr, description, source_url),
        id = "investigate_post",
        retry = 2,
        timeout_secs = 120
    )]
    async fn do_investigate_post(
        website_id: WebsiteId,
        title: String,
        tldr: String,
        description: String,
        source_url: String,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<CrawlEvent> {
        info!(title = %title, source_url = %source_url, "Investigating post");

        let narrative = post_extraction::NarrativePost {
            title,
            tldr,
            description,
            source_url,
        };
        let info = post_extraction::investigate_post(&narrative, ctx.deps())
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "Investigation failed, using defaults");
                crate::common::ExtractedPostInformation::default()
            });

        let post = ExtractedPost::from_narrative_and_info(narrative, info);
        Ok(CrawlEvent::PostInvestigated { website_id, post })
    }

    // =========================================================================
    // Step 4: Join all investigations → sync
    // =========================================================================

    #[effect(on = CrawlEvent, join, id = "join_investigations")]
    async fn join_investigations(
        events: Vec<CrawlEvent>,
        _ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<CrawlEvent> {
        let mut posts = Vec::new();
        let mut website_id = None;

        for event in events {
            if let CrawlEvent::PostInvestigated {
                website_id: wid,
                post,
            } = event
            {
                website_id = Some(wid);
                posts.push(post);
            }
        }

        let website_id = website_id.expect("batch must have items");
        info!(
            website_id = %website_id,
            posts_count = posts.len(),
            "All investigations joined, syncing posts"
        );

        Ok(CrawlEvent::PostsSyncEnqueued { website_id, posts })
    }

    // =========================================================================
    // Step 5: Sync posts to database
    // =========================================================================

    #[effect(
        on = [CrawlEvent::PostsSyncEnqueued],
        extract(website_id, posts),
        id = "sync_posts",
        retry = 2,
        timeout_secs = 120
    )]
    async fn sync_posts(
        website_id: WebsiteId,
        posts: Vec<ExtractedPost>,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        let posts_count = posts.len();

        if posts_count == 0 {
            info!(website_id = %website_id, "No posts to sync, skipping");
            return Ok(());
        }

        info!(
            website_id = %website_id,
            posts_count = posts_count,
            "Syncing posts via LLM (queued effect)"
        );

        let result = llm_sync_posts(
            website_id,
            posts,
            ctx.deps().ai.as_ref(),
            &ctx.deps().db_pool,
        )
        .await?;

        info!(
            website_id = %website_id,
            batch_id = %result.batch_id,
            staged_inserts = result.staged_inserts,
            staged_updates = result.staged_updates,
            staged_deletes = result.staged_deletes,
            staged_merges = result.staged_merges,
            "LLM sync completed - proposals staged for review"
        );

        Ok(())
    }

    // =========================================================================
    // Regeneration paths
    // =========================================================================

    #[effect(
        on = [CrawlEvent::PostsRegenerationEnqueued],
        extract(website_id, visitor_id),
        id = "regenerate_posts",
        retry = 2,
        timeout_secs = 300
    )]
    async fn regenerate_posts_effect(
        website_id: Uuid,
        visitor_id: Uuid,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<CrawlEvent> {
        info!(website_id = %website_id, "Regenerating posts (queued effect)");

        regenerate_posts(website_id, visitor_id, true, ctx.deps()).await?;

        info!(website_id = %website_id, "Regeneration complete, returning extraction enqueue");

        Ok(CrawlEvent::PostsExtractionEnqueued { website_id })
    }

    #[effect(
        on = [CrawlEvent::SinglePostRegenerationEnqueued],
        extract(post_id),
        id = "regenerate_single_post",
        retry = 2,
        timeout_secs = 60
    )]
    async fn regenerate_single_post_effect(
        post_id: Uuid,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        info!(post_id = %post_id, "Regenerating single post (queued effect)");

        regenerate_single_post(post_id, ctx.deps()).await?;

        info!(post_id = %post_id, "Single post regeneration complete");
        Ok(())
    }

    // =========================================================================
    // No-listings handler (moved from crawler.rs)
    // =========================================================================

    #[effect(
        on = [CrawlEvent::WebsiteCrawlNoListings],
        extract(website_id, job_id),
        id = "mark_no_listings"
    )]
    async fn mark_no_listings(
        website_id: WebsiteId,
        job_id: crate::common::JobId,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
        let total_attempts = website.crawl_attempt_count.unwrap_or(0);
        info!(
            website_id = %website_id,
            job_id = %job_id,
            total_attempts = total_attempts,
            "Website marked as having no listings"
        );
        Ok(())
    }
}
