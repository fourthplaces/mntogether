//! Regenerate content for a single page
//!
//! Consolidates the full workflows into single actions.

use tracing::info;
use uuid::Uuid;

use crate::common::JobId;
use crate::domains::crawling::effects::extraction::summarize_pages;
use crate::domains::crawling::models::{PageSnapshot, PageSummary};
use crate::kernel::ServerDeps;

use super::{
    build_page_to_summarize_from_snapshot, extract_posts_from_pages,
    fetch_single_page_context, sync_and_deduplicate_posts,
};

/// Regenerate posts for a single page snapshot.
///
/// Returns the number of posts created/updated, or 0 if anything fails.
/// This consolidates auth-checked workflow into a single action.
pub async fn regenerate_posts_for_page(
    page_snapshot_id: Uuid,
    job_id: JobId,
    deps: &ServerDeps,
) -> usize {
    // Fetch context, build input, extract, sync - chain with early-exit on failure
    let Some(page_ctx) = fetch_single_page_context(page_snapshot_id, &deps.db_pool).await else {
        return 0;
    };

    let page_to_summarize = build_page_to_summarize_from_snapshot(
        &page_ctx.page_snapshot,
        page_ctx.page_snapshot.url.clone(),
    );

    let result = match extract_posts_from_pages(
        &page_ctx.website,
        vec![page_to_summarize],
        job_id,
        deps.ai.as_ref(),
        deps,
    )
    .await
    {
        Ok(r) if !r.posts.is_empty() => r,
        _ => return 0,
    };

    let posts_count = result.posts.len();

    // Sync and deduplicate (ignore errors, we have posts)
    let _ = sync_and_deduplicate_posts(page_ctx.website_id, result.posts, deps).await;

    info!(page_snapshot_id = %page_snapshot_id, posts_count, "Posts regenerated for page");
    posts_count
}

/// Regenerate AI summary for a single page snapshot.
///
/// Returns true if summary was regenerated, false if page not found or failed.
pub async fn regenerate_summary_for_page(
    page_snapshot_id: Uuid,
    deps: &ServerDeps,
) -> bool {
    let page_snapshot = match PageSnapshot::find_by_id(&deps.db_pool, page_snapshot_id).await {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Delete cached summary
    let _ = PageSummary::delete_for_snapshot(page_snapshot_id, &deps.db_pool).await;

    // Build and summarize
    let page_to_summarize = build_page_to_summarize_from_snapshot(&page_snapshot, page_snapshot.url.clone());
    let summaries = summarize_pages(vec![page_to_summarize], deps.ai.as_ref(), &deps.db_pool)
        .await
        .unwrap_or_default();

    let success = !summaries.is_empty();
    info!(page_snapshot_id = %page_snapshot_id, success, "Summary regenerated");
    success
}
