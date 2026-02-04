//! Regenerate content for a single page
//!
//! Uses the extraction library + structured extraction for post generation.

#![allow(deprecated)] // Uses deprecated PageSnapshot during migration

use tracing::{info, warn};
use uuid::Uuid;

use crate::common::JobId;
use crate::domains::crawling::effects::extraction::summarize_pages;
use crate::domains::crawling::models::{PageSnapshot, PageSummary};
use crate::kernel::ServerDeps;

use super::post_extraction::{extract_posts_from_content, POST_SEARCH_QUERY};
use super::{
    build_page_to_summarize_from_snapshot, fetch_single_page_context, sync_and_deduplicate_posts,
};

/// Regenerate posts for a single page snapshot.
///
/// Uses extraction library's extract() + extract_posts_from_content().
/// Returns the number of posts created/updated, or 0 if anything fails.
pub async fn regenerate_posts_for_page(
    page_snapshot_id: Uuid,
    _job_id: JobId,
    deps: &ServerDeps,
) -> usize {
    let Some(page_ctx) = fetch_single_page_context(page_snapshot_id, &deps.db_pool).await else {
        return 0;
    };

    let page_url = page_ctx.page_snapshot.url.clone();
    let website_domain = page_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("");

    info!(page_snapshot_id = %page_snapshot_id, url = %page_url, "Regenerating posts");

    // Search for relevant pages and get raw content
    let pages = match deps
        .extraction
        .search_and_get_pages(POST_SEARCH_QUERY, Some(website_domain), 50)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Search failed");
            return 0;
        }
    };

    if pages.is_empty() {
        info!(page_snapshot_id = %page_snapshot_id, "No relevant pages found");
        return 0;
    }

    // Combine raw page content
    let combined_content: String = pages
        .iter()
        .map(|p| format!("## Source: {}\n\n{}", p.url, p.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    // Extract structured posts
    let context = format!("Source URL: {}", page_url);

    let mut posts = match extract_posts_from_content(
        &combined_content,
        Some(&context),
        deps.ai.as_ref(),
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Structured extraction failed");
            return 0;
        }
    };

    if posts.is_empty() {
        info!(page_snapshot_id = %page_snapshot_id, "No posts extracted");
        return 0;
    }

    // Set source page snapshot ID on all posts
    for post in &mut posts {
        post.source_page_snapshot_id = Some(page_snapshot_id);
    }

    let posts_count = posts.len();

    match sync_and_deduplicate_posts(page_ctx.website_id, posts, deps).await {
        Ok(result) => {
            info!(
                page_snapshot_id = %page_snapshot_id,
                posts_count,
                inserted = result.sync_result.inserted,
                updated = result.sync_result.updated,
                "Posts regenerated"
            );
        }
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Sync failed");
        }
    }

    posts_count
}

/// Regenerate AI summary for a single page snapshot.
pub async fn regenerate_summary_for_page(page_snapshot_id: Uuid, deps: &ServerDeps) -> bool {
    let page_snapshot = match PageSnapshot::find_by_id(&deps.db_pool, page_snapshot_id).await {
        Ok(s) => s,
        Err(_) => return false,
    };

    let _ = PageSummary::delete_for_snapshot(page_snapshot_id, &deps.db_pool).await;

    let page_to_summarize =
        build_page_to_summarize_from_snapshot(&page_snapshot, page_snapshot.url.clone());
    let summaries = summarize_pages(vec![page_to_summarize], deps.ai.as_ref(), &deps.db_pool)
        .await
        .unwrap_or_default();

    let success = !summaries.is_empty();
    info!(page_snapshot_id = %page_snapshot_id, success, "Summary regenerated");
    success
}
