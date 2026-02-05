//! Sync posts action
//!
//! Simple delete-and-replace strategy for post synchronization.

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{ExtractedPost, WebsiteId};
use crate::domains::posts::actions::create_extracted_post;
use crate::kernel::ServerDeps;

/// Result of sync operation.
pub struct SyncAndDedupResult {
    pub sync_result: SyncResult,
    pub deduplicated_count: usize,
}

/// Simple sync result.
pub struct SyncResult {
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
    pub merged: usize,
}

/// Sync extracted posts to database.
///
/// Simple strategy: delete existing posts for the website, insert new ones.
pub async fn sync_and_deduplicate_posts(
    website_id: WebsiteId,
    posts: Vec<ExtractedPost>,
    deps: &ServerDeps,
) -> Result<SyncAndDedupResult> {
    let pool = &deps.db_pool;

    // Delete existing posts for this website
    let deleted = sqlx::query("DELETE FROM posts WHERE website_id = $1")
        .bind(website_id.into_uuid())
        .execute(pool)
        .await?
        .rows_affected() as usize;

    // Insert new posts
    let mut inserted = 0;
    for post in &posts {
        match create_extracted_post(
            &post.title, // organization_name (using title as fallback)
            post,
            Some(website_id),
            post.source_url.clone(),
            pool,
        )
        .await
        {
            Ok(_) => inserted += 1,
            Err(e) => warn!(title = %post.title, error = %e, "Failed to insert post"),
        }
    }

    info!(
        website_id = %website_id,
        deleted,
        inserted,
        "Posts synced (delete and replace)"
    );

    Ok(SyncAndDedupResult {
        sync_result: SyncResult {
            inserted,
            updated: 0,
            deleted,
            merged: 0,
        },
        deduplicated_count: 0,
    })
}
