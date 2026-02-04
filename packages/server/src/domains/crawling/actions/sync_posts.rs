//! Sync posts action
//!
//! Simple delete-and-replace strategy for post synchronization.

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{ExtractedPost, WebsiteId};
use crate::domains::posts::models::Post;
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
        match Post::create(
            post.title.clone(), // organization_name (using title as fallback)
            post.title.clone(),
            post.description.clone(),
            Some(post.tldr.clone()),
            "opportunity".to_string(), // post_type
            "general".to_string(),     // category
            None,                      // capacity_status
            post.urgency.clone(),
            post.location.clone(),
            "active".to_string(),        // status
            "en".to_string(),            // source_language
            Some("scraped".to_string()), // submission_type
            None,                        // submitted_by_admin_id
            Some(website_id),            // website_id
            None,                        // source_url
            None,                        // organization_id
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

/// Deprecated: LLM deduplication no longer used.
#[deprecated(note = "Simple delete-and-replace makes this unnecessary")]
pub async fn llm_deduplicate_website_posts(
    _website_id: WebsiteId,
    _deps: &ServerDeps,
) -> Result<usize> {
    Ok(0)
}
