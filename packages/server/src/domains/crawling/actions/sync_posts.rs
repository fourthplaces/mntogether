//! Sync posts action
//!
//! Sync extracted posts to database and run LLM deduplication.

use anyhow::Result;
use tracing::info;

use crate::common::{ExtractedPost, WebsiteId};
use crate::domains::posts::effects::syncing::{sync_extracted_posts, PostSyncResult};
use crate::domains::posts::effects::deduplication::{apply_dedup_results, deduplicate_posts_llm};
use crate::kernel::ServerDeps;

/// Result of sync and deduplication.
pub struct SyncAndDedupResult {
    pub sync_result: PostSyncResult,
    pub deduplicated_count: usize,
}

/// Sync extracted posts to database and run LLM deduplication.
pub async fn sync_and_deduplicate_posts(
    website_id: WebsiteId,
    posts: Vec<ExtractedPost>,
    deps: &ServerDeps,
) -> Result<SyncAndDedupResult> {
    // Sync posts using existing logic (title-match only, no embedding dedup)
    let sync_result = sync_extracted_posts(website_id, posts, &deps.db_pool).await?;

    info!(
        website_id = %website_id,
        new_count = sync_result.new_count,
        updated_count = sync_result.updated_count,
        unchanged_count = sync_result.unchanged_count,
        "Sync completed"
    );

    // Run LLM-based deduplication
    let deduplicated_count = llm_deduplicate_website_posts(website_id, deps).await.unwrap_or(0);

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
