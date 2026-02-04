//! Post revision management actions
//!
//! Handles approving and rejecting revision posts created during LLM sync.
//! Revisions are draft posts that reference an original via `revision_of_post_id`.

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::common::{PostId, WebsiteId};
use crate::domains::posts::models::Post;

/// Approve revision: copy revision fields to original, delete revision
///
/// Returns the updated original post.
pub async fn approve_revision(revision_id: PostId, pool: &PgPool) -> Result<Post> {
    let revision = Post::find_by_id(revision_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Revision not found"))?;

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
        original_id,
        Some(revision.title),
        Some(revision.description),
        revision.description_markdown,
        revision.tldr,
        Some(revision.category),
        revision.urgency,
        revision.location,
        pool,
    )
    .await?;

    // Delete the revision
    Post::delete(revision_id, pool).await?;

    info!(
        original_id = %original_id,
        "Revision approved and applied"
    );

    Ok(updated)
}

/// Reject revision: delete revision, original unchanged
pub async fn reject_revision(revision_id: PostId, pool: &PgPool) -> Result<()> {
    let revision = Post::find_by_id(revision_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Revision not found"))?;

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

/// Get all pending revisions, optionally filtered by website
pub async fn get_pending_revisions(
    website_id: Option<WebsiteId>,
    pool: &PgPool,
) -> Result<Vec<Post>> {
    match website_id {
        Some(id) => Post::find_revisions_by_website(id, pool).await,
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
