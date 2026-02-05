//! Individual sync operation functions
//!
//! Extracted from the monolithic apply_sync_operations for single-responsibility.
//! Each function handles one operation type and can be unit tested independently.

use sqlx::PgPool;
use tracing::info;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::common::{ExtractedPost, PostId, WebsiteId};
use crate::domains::posts::models::{Post, UpdatePostContent};
use crate::domains::website::models::Website;

use super::create_post::create_extracted_post;

/// Arguments for merge operation using typed builder pattern
#[derive(TypedBuilder)]
pub struct MergeArgs<'a> {
    pub canonical_id: &'a str,
    pub canonical: &'a Post,
    pub duplicate_ids: &'a [String],
    pub existing_by_id: &'a std::collections::HashMap<String, &'a Post>,
    #[builder(default)]
    pub merged_title: Option<&'a str>,
    #[builder(default)]
    pub merged_description: Option<&'a str>,
    pub reason: &'a str,
    pub pool: &'a PgPool,
}

/// Result of a single sync operation
#[derive(Debug)]
pub enum SyncOpResult {
    /// Post was inserted, returns new post ID
    Inserted(PostId),
    /// Post was updated
    Updated(PostId),
    /// Post was deleted
    Deleted(PostId),
    /// Duplicate was merged (deleted into canonical)
    Merged { canonical: PostId, deleted: PostId },
    /// Operation was skipped (e.g., protected post)
    Skipped,
    /// Operation failed with error message
    Error(String),
}

/// Apply a single INSERT operation
///
/// Creates a new post from fresh extraction data.
pub async fn apply_insert(
    fresh_id: &str,
    fresh: &ExtractedPost,
    website_id: WebsiteId,
    pool: &PgPool,
) -> SyncOpResult {
    info!(
        action = "INSERTING",
        fresh_id = %fresh_id,
        title = %fresh.title,
        "Inserting new post into database"
    );

    let website = match Website::find_by_id(website_id, pool).await {
        Ok(w) => w,
        Err(e) => return SyncOpResult::Error(format!("Failed to load website: {}", e)),
    };

    match create_extracted_post(
        &website.domain,
        fresh,
        Some(website_id),
        fresh.source_url.clone().or_else(|| Some(format!("https://{}", website.domain))),
        pool,
    )
    .await
    {
        Ok(post) => {
            info!(
                action = "INSERTED",
                post_id = %post.id,
                title = %fresh.title,
                "Successfully inserted new post"
            );
            SyncOpResult::Inserted(post.id)
        }
        Err(e) => {
            tracing::error!(
                action = "INSERT_FAILED",
                fresh_id = %fresh_id,
                title = %fresh.title,
                error = %e,
                "Failed to insert post"
            );
            SyncOpResult::Error(format!("Insert {}: {}", fresh_id, e))
        }
    }
}

/// Apply a single UPDATE operation
///
/// Updates an existing post with fresh extraction data by creating a revision.
pub async fn apply_update(
    _fresh_id: &str,
    existing_id: &str,
    existing: &Post,
    fresh: &ExtractedPost,
    merge_description: bool,
    pool: &PgPool,
) -> SyncOpResult {
    info!(
        action = "UPDATING",
        post_id = %existing.id,
        old_title = %existing.title,
        new_title = %fresh.title,
        "Updating existing post"
    );

    match super::llm_sync::update_post(existing.id, fresh, merge_description, pool).await {
        Ok(_) => {
            info!(
                action = "UPDATED",
                post_id = %existing.id,
                title = %fresh.title,
                "Successfully updated post"
            );
            SyncOpResult::Updated(existing.id)
        }
        Err(e) => {
            tracing::error!(
                action = "UPDATE_FAILED",
                post_id = %existing_id,
                error = %e,
                "Failed to update post"
            );
            SyncOpResult::Error(format!("Update {}: {}", existing_id, e))
        }
    }
}

/// Apply a single DELETE operation
///
/// Soft-deletes a post that no longer exists in the fresh extraction.
/// Protects active and pending_approval posts from deletion.
pub async fn apply_delete(
    existing_id: &str,
    existing: &Post,
    reason: &str,
    pool: &PgPool,
) -> SyncOpResult {
    // Only delete posts that are explicitly rejected or expired
    // Protect active AND pending_approval posts from accidental deletion
    let protected_statuses = ["active", "pending_approval"];
    if protected_statuses.contains(&existing.status.as_str()) {
        info!(
            action = "DELETE_SKIPPED",
            post_id = %existing_id,
            title = %existing.title,
            status = %existing.status,
            "Skipping delete of protected post (only rejected/expired can be deleted)"
        );
        return SyncOpResult::Skipped;
    }

    info!(
        action = "DELETING",
        post_id = %existing_id,
        title = %existing.title,
        reason = %reason,
        "Soft-deleting stale post"
    );

    match Post::soft_delete(existing.id, reason, pool).await {
        Ok(_) => {
            info!(
                action = "DELETED",
                post_id = %existing_id,
                title = %existing.title,
                "Successfully soft-deleted post"
            );
            SyncOpResult::Deleted(existing.id)
        }
        Err(e) => {
            tracing::error!(
                action = "DELETE_FAILED",
                post_id = %existing_id,
                error = %e,
                "Failed to delete post"
            );
            SyncOpResult::Error(format!("Delete {}: {}", existing_id, e))
        }
    }
}

/// Update canonical post with merged content
async fn update_canonical_content(
    canonical_id: &str,
    canonical: &Post,
    merged_title: Option<&str>,
    merged_description: Option<&str>,
    pool: &PgPool,
) {
    if merged_title.is_none() && merged_description.is_none() {
        return;
    }

    info!(
        action = "MERGE_UPDATE_CANONICAL",
        post_id = %canonical_id,
        new_title = ?merged_title,
        "Updating canonical post with merged content"
    );

    let _ = Post::update_content(
        UpdatePostContent::builder()
            .id(canonical.id)
            .title(merged_title.map(String::from))
            .description(merged_description.map(String::from))
            .build(),
        pool,
    )
    .await;
}

/// Delete a single duplicate post during merge
async fn delete_duplicate(
    dup_id: &str,
    canonical_id: &str,
    canonical_post_id: PostId,
    existing_by_id: &std::collections::HashMap<String, &Post>,
    reason: &str,
    pool: &PgPool,
) -> SyncOpResult {
    let Ok(id) = Uuid::parse_str(dup_id) else {
        return SyncOpResult::Error(format!("Invalid UUID: {}", dup_id));
    };
    let post_id = PostId::from(id);

    let dup_title = existing_by_id.get(dup_id).map(|e| e.title.as_str()).unwrap_or("?");

    // Don't delete active posts
    if let Some(existing) = existing_by_id.get(dup_id) {
        if existing.status == "active" {
            info!(action = "MERGE_SKIP_ACTIVE", dup_id = %dup_id, dup_title = %dup_title, "Skipping merge of active duplicate");
            return SyncOpResult::Skipped;
        }
    }

    info!(action = "MERGE_DELETE_DUP", dup_id = %dup_id, dup_title = %dup_title, canonical_id = %canonical_id, "Deleting duplicate post");

    match Post::soft_delete(post_id, reason, pool).await {
        Ok(_) => {
            info!(action = "MERGE_DELETED", dup_id = %dup_id, dup_title = %dup_title, "Successfully merged (deleted) duplicate");
            SyncOpResult::Merged { canonical: canonical_post_id, deleted: post_id }
        }
        Err(e) => {
            tracing::error!(action = "MERGE_DELETE_FAILED", dup_id = %dup_id, error = %e, "Failed to delete duplicate");
            SyncOpResult::Error(format!("Merge {}: {}", dup_id, e))
        }
    }
}

/// Apply a single MERGE operation
///
/// Thin orchestrator that delegates to helper functions.
pub async fn apply_merge(args: MergeArgs<'_>) -> Vec<SyncOpResult> {
    info!(
        action = "MERGING",
        canonical_id = %args.canonical_id,
        canonical_title = %args.canonical.title,
        duplicate_count = args.duplicate_ids.len(),
        reason = %args.reason,
        "Merging duplicate posts"
    );

    update_canonical_content(
        args.canonical_id,
        args.canonical,
        args.merged_title,
        args.merged_description,
        args.pool,
    )
    .await;

    let mut results = Vec::with_capacity(args.duplicate_ids.len());
    for dup_id in args.duplicate_ids {
        let result = delete_duplicate(
            dup_id,
            args.canonical_id,
            args.canonical.id,
            args.existing_by_id,
            args.reason,
            args.pool,
        )
        .await;
        results.push(result);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_op_result_variants() {
        // Just verify the enum compiles and variants are accessible
        let _inserted = SyncOpResult::Inserted(PostId::from(Uuid::new_v4()));
        let _updated = SyncOpResult::Updated(PostId::from(Uuid::new_v4()));
        let _deleted = SyncOpResult::Deleted(PostId::from(Uuid::new_v4()));
        let _merged = SyncOpResult::Merged {
            canonical: PostId::from(Uuid::new_v4()),
            deleted: PostId::from(Uuid::new_v4()),
        };
        let _skipped = SyncOpResult::Skipped;
        let _error = SyncOpResult::Error("test error".to_string());
    }
}
