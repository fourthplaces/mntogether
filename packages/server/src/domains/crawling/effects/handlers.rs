//! Cascade event handlers for crawling domain
//!
//! These handlers implement the effect cascade for crawling workflows:
//! - `handle_extract_posts_from_pages`: Extract posts from ingested pages
//! - `handle_sync_crawled_posts`: Sync extracted posts to database
//! - `handle_mark_no_posts`: Mark website as having no listings

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{AppState, ExtractedPost, JobId, WebsiteId};
use crate::domains::crawling::actions::post_extraction::extract_posts_for_domain;
use crate::domains::crawling::actions::sync_and_deduplicate_posts;
use crate::domains::crawling::events::{CrawlEvent, PageExtractionResult};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

// ============================================================================
// Post Extraction Handler
// ============================================================================

/// Extract posts from pages using the extraction library.
///
/// Uses search + raw page fetch to get comprehensive content, then
/// structured extraction to parse directly to Vec<ExtractedPost>.
///
/// This approach separates retrieval (finding relevant pages) from extraction
/// (parsing structured posts), avoiding the double-summarization issue where
/// RAG-summarized content loses important details.
pub async fn handle_extract_posts_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, job_id = %job_id, "Starting post extraction");

    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;

    // Extraction service is required for post extraction
    let extraction = ctx
        .deps()
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // Search and extract posts using shared action
    let result = extract_posts_for_domain(&website.domain, extraction.as_ref(), ctx.deps()).await?;

    if result.posts.is_empty() && result.page_urls.is_empty() {
        info!(website_id = %website_id, "No relevant pages found");
        return Ok(());
    }

    // Build page results for the event
    let page_results: Vec<PageExtractionResult> = result
        .page_urls
        .iter()
        .map(|url| PageExtractionResult {
            url: url.clone(),
            snapshot_id: None,
            listings_count: 0,
            has_posts: true,
        })
        .collect();

    info!(
        website_id = %website_id,
        posts_count = result.posts.len(),
        "Post extraction completed"
    );

    ctx.emit(CrawlEvent::PostsExtractedFromPages {
        website_id,
        job_id,
        posts: result.posts,
        page_results,
    });

    Ok(())
}

/// Mark website as having no posts.
pub async fn handle_mark_no_posts(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, "Marking website as having no posts");

    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
    let total_attempts = website.crawl_attempt_count.unwrap_or(0);

    ctx.emit(CrawlEvent::WebsiteMarkedNoListings {
        website_id,
        job_id,
        total_attempts,
    });
    Ok(())
}

/// Sync posts to database.
pub async fn handle_sync_crawled_posts(
    website_id: WebsiteId,
    job_id: JobId,
    posts: Vec<ExtractedPost>,
    _page_results: Vec<PageExtractionResult>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(website_id = %website_id, posts_count = posts.len(), "Syncing posts");

    let result = sync_and_deduplicate_posts(website_id, posts, ctx.deps()).await?;

    ctx.emit(CrawlEvent::PostsSynced {
        website_id,
        job_id,
        new_count: result.sync_result.inserted,
        updated_count: result.sync_result.updated,
        unchanged_count: result.sync_result.deleted + result.sync_result.merged,
    });
    Ok(())
}
