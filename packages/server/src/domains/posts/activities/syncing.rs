// Domain functions for listing synchronization
//
// These functions contain business logic for syncing extracted listings
// with the database, separated from the thin Effect orchestrator.
//
// NOTE: Deduplication is now handled by LLM-based deduplication after sync.
// See `crate::domains::posts::activities::deduplication`.

use anyhow::{Context, Result};
use sqlx::PgPool;

use super::sync_utils::{sync_posts, ExtractedPostInput};
use crate::common::ExtractedPost;
use crate::common::WebsiteId;
use crate::domains::website::models::Website;

/// Result of syncing listings with the database
pub struct PostSyncResult {
    pub new_count: usize,
    pub updated_count: usize,
    pub unchanged_count: usize,
}

/// Sync extracted listings with the database for a given source
///
/// This function:
/// 1. Fetches the source website
/// 2. Converts extracted listings to sync input format
/// 3. Performs sync operation with database (title-match only)
/// 4. Returns summary of changes
///
/// NOTE: Deduplication is handled separately by LLM-based deduplication.
/// This function only does title-matching for updates.
pub async fn sync_extracted_posts(
    source_id: WebsiteId,
    posts: Vec<ExtractedPost>,
    pool: &PgPool,
) -> Result<PostSyncResult> {
    let source = Website::find_by_id(source_id, pool)
        .await
        .context("Failed to find source")?;

    // Convert event listings to sync input
    let sync_input: Vec<ExtractedPostInput> = posts
        .into_iter()
        .map(|listing| ExtractedPostInput {
            title: listing.title,
            description: listing.description,
            description_markdown: None,
            summary: Some(listing.summary),
            contact: listing.contact.and_then(|c| {
                serde_json::json!({
                    "email": c.email,
                    "phone": c.phone,
                    "website": c.website
                })
                .as_object()
                .map(|obj| serde_json::Value::Object(obj.clone()))
            }),
            location: listing.location,
            urgency: listing.urgency,
            confidence: listing.confidence,
            source_url: Some(format!("https://{}", source.domain)), // Use domain as URL
            audience_roles: listing.audience_roles, // Pass through extracted audience roles
            tags: listing.tags,
        })
        .collect();

    // Sync with database (title-match only - LLM handles semantic dedup after)
    let website_id = WebsiteId::from_uuid(source_id.into_uuid());
    let sync_result = sync_posts(pool, website_id, sync_input)
        .await
        .context("Sync failed")?;

    Ok(PostSyncResult {
        new_count: sync_result.new_posts.len(),
        updated_count: sync_result.updated_posts.len(),
        unchanged_count: sync_result.unchanged_posts.len(),
    })
}
