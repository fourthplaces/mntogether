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
use crate::domains::crawling::actions::post_extraction::{
    extract_posts_from_content, POST_SEARCH_QUERY,
};
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
/// extract_posts_from_content() to parse directly to Vec<ExtractedPost>.
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

    // Use search + raw fetch (NOT extract which summarizes)
    // This gives us full page content for comprehensive extraction
    info!(
        website_id = %website_id,
        domain = %website.domain,
        query = POST_SEARCH_QUERY,
        "Searching for pages"
    );

    let pages = ctx
        .deps()
        .extraction
        .search_and_get_pages(POST_SEARCH_QUERY, Some(&website.domain), 50)
        .await?;

    info!(
        website_id = %website_id,
        pages_found = pages.len(),
        page_urls = ?pages.iter().map(|p| &p.url).collect::<Vec<_>>(),
        "Search results"
    );

    if pages.is_empty() {
        info!(website_id = %website_id, "No relevant pages found");
        return Ok(());
    }

    // Combine raw page content (not summaries)
    let combined_content: String = pages
        .iter()
        .map(|p| format!("## Source: {}\n\n{}", p.url, p.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let page_results: Vec<PageExtractionResult> = pages
        .iter()
        .map(|p| PageExtractionResult {
            url: p.url.clone(),
            snapshot_id: None,
            listings_count: 0,
            has_posts: true,
        })
        .collect();

    info!(
        website_id = %website_id,
        pages_count = pages.len(),
        content_len = combined_content.len(),
        "Extracting structured posts from raw content"
    );

    // Extract structured posts from raw content
    let context = format!(
        "Organization: {}\nSource URL: https://{}",
        website.domain, website.domain
    );

    let posts =
        extract_posts_from_content(&combined_content, Some(&context), ctx.deps().ai.as_ref())
            .await?;

    info!(
        website_id = %website_id,
        posts_count = posts.len(),
        "Post extraction completed"
    );

    ctx.emit(CrawlEvent::PostsExtractedFromPages {
        website_id,
        job_id,
        posts,
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
