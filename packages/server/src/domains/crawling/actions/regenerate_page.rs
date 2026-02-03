//! Regenerate content for a single page
//!
//! Uses agentic extraction for post generation.
//!
//! # Deprecation Notice
//!
//! This module uses the deprecated `PageSnapshot` model. For new code, use
//! the extraction library's `ExtractionService` which stores pages in
//! `extraction_pages` and handles extraction via `Index::extract()`.

#![allow(deprecated)] // Uses deprecated PageSnapshot during migration

use tracing::{info, warn};
use uuid::Uuid;

use crate::common::JobId;
use crate::domains::crawling::effects::extraction::summarize_pages;
use crate::domains::crawling::models::{PageSnapshot, PageSummary};
use crate::domains::posts::effects::agentic_extraction::{
    extract_from_website, to_extracted_posts,
};
use crate::kernel::ServerDeps;

use super::{
    build_page_to_summarize_from_snapshot, fetch_single_page_context, sync_and_deduplicate_posts,
};

/// Regenerate posts for a single page snapshot using agentic extraction.
///
/// Uses the same agentic extraction as the main crawl pipeline for consistency.
/// Returns the number of posts created/updated, or 0 if anything fails.
pub async fn regenerate_posts_for_page(
    page_snapshot_id: Uuid,
    _job_id: JobId,
    deps: &ServerDeps,
) -> usize {
    // Fetch context
    let Some(page_ctx) = fetch_single_page_context(page_snapshot_id, &deps.db_pool).await else {
        return 0;
    };

    // Get page content directly from snapshot
    let page_content = page_ctx
        .page_snapshot
        .markdown
        .clone()
        .unwrap_or_else(|| page_ctx.page_snapshot.html.clone());

    let page_url = page_ctx.page_snapshot.url.clone();

    info!(page_snapshot_id = %page_snapshot_id, url = %page_url, "Regenerating posts with agentic extraction");

    // Build single-page input for agentic extraction
    let pages = vec![(page_snapshot_id, page_url.clone(), page_content)];

    // Run agentic extraction
    let extraction_result = match extract_from_website(
        page_ctx.website_id,
        &pages,
        &deps.db_pool,
        Some(deps.web_searcher.as_ref()),
        deps.ai.as_ref(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Agentic extraction failed");
            return 0;
        }
    };

    if extraction_result.posts.is_empty() {
        info!(page_snapshot_id = %page_snapshot_id, "No posts extracted");
        return 0;
    }

    // Convert to ExtractedPost format and sync
    let posts = to_extracted_posts(&extraction_result.posts);
    let posts_count = posts.len();

    info!(
        page_snapshot_id = %page_snapshot_id,
        posts_count,
        "Starting sync for extracted posts"
    );

    match sync_and_deduplicate_posts(page_ctx.website_id, posts, deps).await {
        Ok(result) => {
            info!(
                page_snapshot_id = %page_snapshot_id,
                posts_count,
                inserted = result.sync_result.inserted,
                updated = result.sync_result.updated,
                deleted = result.sync_result.deleted,
                merged = result.sync_result.merged,
                candidates_found = extraction_result.candidates_found,
                candidates_skipped = extraction_result.candidates_skipped,
                "Posts regenerated with agentic extraction"
            );
        }
        Err(e) => {
            warn!(
                page_snapshot_id = %page_snapshot_id,
                error = %e,
                posts_count,
                "Sync failed for regenerated posts"
            );
        }
    }

    posts_count
}

/// Regenerate AI summary for a single page snapshot.
///
/// Returns true if summary was regenerated, false if page not found or failed.
pub async fn regenerate_summary_for_page(page_snapshot_id: Uuid, deps: &ServerDeps) -> bool {
    let page_snapshot = match PageSnapshot::find_by_id(&deps.db_pool, page_snapshot_id).await {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Delete cached summary
    let _ = PageSummary::delete_for_snapshot(page_snapshot_id, &deps.db_pool).await;

    // Build and summarize
    let page_to_summarize =
        build_page_to_summarize_from_snapshot(&page_snapshot, page_snapshot.url.clone());
    let summaries = summarize_pages(vec![page_to_summarize], deps.ai.as_ref(), &deps.db_pool)
        .await
        .unwrap_or_default();

    let success = !summaries.is_empty();
    info!(page_snapshot_id = %page_snapshot_id, success, "Summary regenerated");
    success
}
