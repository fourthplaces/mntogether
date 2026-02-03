//! Sync posts action
//!
//! Uses LLM-powered sync to intelligently handle INSERT/UPDATE/DELETE/MERGE
//! in a single pass, avoiding duplicates.

use anyhow::Result;
use tracing::info;

use crate::common::{ExtractedPost, WebsiteId};
use crate::domains::posts::effects::deduplication::{apply_dedup_results, deduplicate_posts_llm};
use crate::domains::posts::effects::llm_sync::{llm_sync_posts, SyncResult};
use crate::kernel::ServerDeps;

/// Result of sync and deduplication.
pub struct SyncAndDedupResult {
    pub sync_result: SyncResult,
    pub deduplicated_count: usize,
}

/// Sync extracted posts to database using LLM-powered intelligent sync.
///
/// The LLM analyzes fresh posts vs existing DB posts and decides:
/// - INSERT: New posts that don't exist
/// - UPDATE: Fresh posts that match existing (semantically)
/// - DELETE: DB posts no longer in fresh extraction
/// - MERGE: Consolidate pre-existing duplicates
pub async fn sync_and_deduplicate_posts(
    website_id: WebsiteId,
    posts: Vec<ExtractedPost>,
    deps: &ServerDeps,
) -> Result<SyncAndDedupResult> {
    // Use LLM-powered sync that handles INSERT/UPDATE/DELETE/MERGE in one pass
    let sync_result = llm_sync_posts(website_id, posts, deps.ai.as_ref(), &deps.db_pool).await?;

    info!(
        website_id = %website_id,
        inserted = sync_result.inserted,
        updated = sync_result.updated,
        deleted = sync_result.deleted,
        merged = sync_result.merged,
        "LLM sync completed"
    );

    // The LLM sync already handles deduplication via MERGE operations,
    // but we can run a second pass to catch any edge cases
    let deduplicated_count = if sync_result.merged == 0 {
        // Only run extra dedup if LLM sync didn't merge anything
        llm_deduplicate_website_posts(website_id, deps)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    Ok(SyncAndDedupResult {
        sync_result,
        deduplicated_count,
    })
}

/// Run LLM-based deduplication for a website's posts.
///
/// Returns the number of duplicate posts soft-deleted.
pub async fn llm_deduplicate_website_posts(
    website_id: WebsiteId,
    deps: &ServerDeps,
) -> Result<usize> {
    // Run LLM deduplication analysis
    let dedup_result = deduplicate_posts_llm(website_id, deps.ai.as_ref(), &deps.db_pool).await?;

    // Apply the results (soft-delete duplicates)
    let deleted_count = apply_dedup_results(dedup_result, deps.ai.as_ref(), &deps.db_pool).await?;

    if deleted_count > 0 {
        info!(
            website_id = %website_id,
            deleted_count = deleted_count,
            "LLM deduplication completed"
        );
    }

    Ok(deleted_count)
}
