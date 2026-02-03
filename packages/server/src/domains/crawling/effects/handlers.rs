//! Cascade event handlers for crawling domain
//!
//! These handlers respond to INTERNAL CASCADE events in the multi-step crawling workflow.
//! They are called by the effect dispatcher, NOT from GraphQL directly.
//!
//! Pattern:
//!   Fact Event → Effect → Cascade Handler → do work → emit next event

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{AppState, ExtractedPost, JobId, MemberId, WebsiteId};
use crate::domains::crawling::actions::{
    build_pages_to_summarize, crawl_website, sync_and_deduplicate_posts,
};
use crate::domains::crawling::events::{CrawlEvent, CrawledPageInfo, PageExtractionResult};
use crate::domains::posts::effects::agentic_extraction::{
    extract_from_website, to_extracted_posts,
};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

// ============================================================================
// Handler: Extract Posts From Pages (cascade from WebsiteCrawled)
// ============================================================================

/// Handle extract posts request - cascade handler.
/// Uses agentic extraction with tool-calling for rich post enrichment.
pub async fn handle_extract_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    pages: Vec<CrawledPageInfo>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, pages_count = pages.len(), "Extracting posts (agentic)");

    // Build pages with content for agentic extraction
    let (pages_to_summarize, _) = build_pages_to_summarize(&pages, &ctx.deps().db_pool).await?;

    if pages_to_summarize.is_empty() {
        ctx.emit(CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: 1,
            pages_crawled: 0,
            should_retry: false,
        });
        return Ok(());
    }

    // Convert to format expected by agentic extraction: (page_snapshot_id, url, content)
    let pages_for_extraction: Vec<(uuid::Uuid, String, String)> = pages_to_summarize
        .iter()
        .map(|p| (p.snapshot_id, p.url.clone(), p.raw_content.clone()))
        .collect();

    // Run agentic extraction with tool-calling
    let extraction_result = match extract_from_website(
        website_id,
        &pages_for_extraction,
        &ctx.deps().db_pool,
        Some(ctx.deps().search_service.as_ref()),
        ctx.deps().ai.as_ref(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(website_id = %website_id, error = %e, "Agentic extraction failed");
            ctx.emit(CrawlEvent::WebsiteCrawlNoListings {
                website_id,
                job_id,
                attempt_number: 1,
                pages_crawled: pages.len(),
                should_retry: true,
            });
            return Ok(());
        }
    };

    // Convert EnrichedPosts to ExtractedPosts for pipeline compatibility
    let extracted_posts = to_extracted_posts(&extraction_result.posts);

    // Build page results from extraction
    let page_results: Vec<PageExtractionResult> = pages_to_summarize
        .iter()
        .map(|p| {
            // Count posts that came from this page
            let posts_from_page = extraction_result
                .posts
                .iter()
                .filter(|post| post.source_page_snapshot_id == Some(p.snapshot_id))
                .count();

            PageExtractionResult {
                url: p.url.clone(),
                snapshot_id: Some(p.snapshot_id),
                listings_count: posts_from_page,
                has_posts: posts_from_page > 0,
            }
        })
        .collect();

    // Check if we found any posts
    if extracted_posts.is_empty() {
        let attempt_count = Website::increment_crawl_attempt(website_id, &ctx.deps().db_pool)
            .await
            .unwrap_or(1);
        let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
        ctx.emit(CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: attempt_count,
            pages_crawled: pages.len(),
            should_retry: website.should_retry_crawl(),
        });
        return Ok(());
    }

    info!(
        website_id = %website_id,
        total_posts = extracted_posts.len(),
        candidates_found = extraction_result.candidates_found,
        candidates_skipped = extraction_result.candidates_skipped,
        posts_merged = extraction_result.posts_merged,
        "Agentic extraction complete"
    );
    let _ = Website::reset_crawl_attempts(website_id, &ctx.deps().db_pool).await;

    ctx.emit(CrawlEvent::PostsExtractedFromPages {
        website_id,
        job_id,
        posts: extracted_posts,
        page_results,
    });
    Ok(())
}

// ============================================================================
// Handler: Retry Website Crawl (cascade from WebsiteCrawlNoListings when should_retry=true)
// ============================================================================

/// Handle retry crawl request - cascade handler.
pub async fn handle_retry_crawl(
    website_id: WebsiteId,
    _job_id: JobId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, "Retrying website crawl");
    let _ = Website::reset_for_retry(website_id, &ctx.deps().db_pool).await;

    // Call the crawl_website action directly (convert typed IDs to Uuids)
    let _ = crawl_website(
        website_id.into_uuid(),
        MemberId::new().into_uuid(), // System retry
        true,                        // is_admin
        ctx,
    )
    .await;

    Ok(())
}

// ============================================================================
// Handler: Mark Website No Posts (cascade from WebsiteCrawlNoListings when should_retry=false)
// ============================================================================

/// Handle mark no posts request - cascade handler.
pub async fn handle_mark_no_posts(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, "Marking website as having no posts");

    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
    let total_attempts = website.crawl_attempt_count.unwrap_or(0);

    let _ = Website::complete_crawl(
        website_id,
        "no_posts_found",
        website.pages_crawled_count.unwrap_or(0),
        &ctx.deps().db_pool,
    )
    .await;

    ctx.emit(CrawlEvent::WebsiteMarkedNoListings {
        website_id,
        job_id,
        total_attempts,
    });
    Ok(())
}

// ============================================================================
// Handler: Sync Crawled Posts (cascade from PostsExtractedFromPages)
// ============================================================================

/// Handle sync crawled posts request - cascade handler.
pub async fn handle_sync_crawled_posts(
    website_id: WebsiteId,
    job_id: JobId,
    posts: Vec<ExtractedPost>,
    page_results: Vec<PageExtractionResult>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, posts_count = posts.len(), "Syncing crawled posts");

    // Sync and deduplicate
    let result = sync_and_deduplicate_posts(website_id, posts, ctx.deps()).await?;

    // Mark crawl as completed
    let _ = Website::complete_crawl(
        website_id,
        "completed",
        page_results.len() as i32,
        &ctx.deps().db_pool,
    )
    .await;

    ctx.emit(CrawlEvent::PostsSynced {
        website_id,
        job_id,
        new_count: result.sync_result.inserted,
        updated_count: result.sync_result.updated,
        // LLM sync doesn't track unchanged - use deleted + merged count as info
        unchanged_count: result.sync_result.deleted + result.sync_result.merged,
    });
    Ok(())
}
