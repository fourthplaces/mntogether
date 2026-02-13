//! Post revision management actions
//!
//! Handles approving and rejecting revision posts created during LLM sync.
//! Revisions are draft posts that reference an original via `revision_of_post_id`.

use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::common::PostId;
use crate::domains::posts::models::{Post, UpdatePostContent};
use uuid::Uuid;

/// Approve revision: copy revision fields to original, delete revision
///
/// Returns the updated original post, or None if the revision was already applied.
/// Idempotent: if the revision no longer exists, it was already consumed by a prior attempt.
pub async fn approve_revision(revision_id: PostId, pool: &PgPool) -> Result<Option<Post>> {
    let revision = match Post::find_by_id(revision_id, pool).await? {
        Some(r) => r,
        None => {
            warn!(revision_id = %revision_id, "Revision not found - already applied, skipping");
            return Ok(None);
        }
    };

    let original_id = revision
        .revision_of_post_id
        .ok_or_else(|| anyhow::anyhow!("Not a revision post"))?;

    info!(
        revision_id = %revision_id,
        original_id = %original_id,
        title = %revision.title,
        "Approving revision - copying to original"
    );

    // Copy revision fields to original
    let updated = Post::update_content(
        UpdatePostContent::builder()
            .id(original_id)
            .title(Some(revision.title))
            .description(Some(revision.description))
            .description_markdown(revision.description_markdown)
            .summary(revision.summary)
            .category(Some(revision.category))
            .urgency(revision.urgency)
            .location(revision.location)
            .build(),
        pool,
    )
    .await?;

    // Delete the revision
    Post::delete(revision_id, pool).await?;

    info!(
        original_id = %original_id,
        "Revision approved and applied"
    );

    Ok(Some(updated))
}

/// Reject revision: delete revision, original unchanged.
/// Idempotent: if the revision no longer exists, it was already deleted.
pub async fn reject_revision(revision_id: PostId, pool: &PgPool) -> Result<()> {
    let revision = match Post::find_by_id(revision_id, pool).await? {
        Some(r) => r,
        None => {
            warn!(revision_id = %revision_id, "Revision not found - already deleted, skipping");
            return Ok(());
        }
    };

    if revision.revision_of_post_id.is_none() {
        anyhow::bail!("Not a revision post");
    }

    info!(
        revision_id = %revision_id,
        original_id = ?revision.revision_of_post_id,
        title = %revision.title,
        "Rejecting revision - deleting"
    );

    Post::delete(revision_id, pool).await
}

/// Get all pending revisions, optionally filtered by source
pub async fn get_pending_revisions(
    source: Option<(&str, Uuid)>,
    pool: &PgPool,
) -> Result<Vec<Post>> {
    match source {
        Some((source_type, source_id)) => {
            Post::find_revisions_by_source(source_type, source_id, pool).await
        }
        None => Post::find_pending_revisions(pool).await,
    }
}

/// Get the revision for a specific original post (if any exists)
pub async fn get_revision_for_post(post_id: PostId, pool: &PgPool) -> Result<Option<Post>> {
    Post::find_revision_for_post(post_id, pool).await
}

/// Count pending revisions
pub async fn count_pending_revisions(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM posts
        WHERE revision_of_post_id IS NOT NULL
          AND deleted_at IS NULL
          AND status = 'pending_approval'
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(count)
}
