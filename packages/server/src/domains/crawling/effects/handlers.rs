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
    build_pages_to_summarize, crawl_website, extract_posts_from_pages, sync_and_deduplicate_posts,
    update_page_extraction_status,
};
use crate::domains::crawling::events::{CrawledPageInfo, CrawlEvent, PageExtractionResult};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

// ============================================================================
// Handler: Extract Posts From Pages (cascade from WebsiteCrawled)
// ============================================================================

/// Handle extract posts request - cascade handler.
pub async fn handle_extract_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    pages: Vec<CrawledPageInfo>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, pages_count = pages.len(), "Extracting posts");

    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;

    // Build pages to summarize
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

    // Extract posts using two-pass extraction
    let result = match extract_posts_from_pages(
        &website,
        pages_to_summarize,
        job_id,
        ctx.deps().ai.as_ref(),
        ctx.deps(),
    )
    .await
    {
        Ok(r) => r,
        Err(event) => {
            ctx.emit(event);
            return Ok(());
        }
    };

    // Update page extraction status
    update_page_extraction_status(&result.page_results, &ctx.deps().db_pool).await;

    // Check if we found any posts
    if result.posts.is_empty() {
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

    info!(website_id = %website_id, total_posts = result.posts.len(), "Extraction complete");
    let _ = Website::reset_crawl_attempts(website_id, &ctx.deps().db_pool).await;

    ctx.emit(CrawlEvent::PostsExtractedFromPages {
        website_id,
        job_id,
        posts: result.posts,
        page_results: result.page_results,
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
        new_count: result.sync_result.new_count,
        updated_count: result.sync_result.updated_count,
        unchanged_count: result.sync_result.unchanged_count,
    });
    Ok(())
}
