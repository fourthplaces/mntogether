//! Note attachment activities.
//!
//! Links active org-level notes to posts. Two entry points:
//! - `attach_notes_to_org_posts`: bulk-attach all active org notes to all active org posts
//! - `attach_org_notes_to_post`: attach all active org notes to a single new post

use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::common::{OrganizationId, PostId};
use crate::domains::notes::models::{Note, Noteable};
use crate::domains::posts::models::Post;

pub struct AttachResult {
    pub notes_count: i32,
    pub posts_count: i32,
    pub noteables_created: i32,
}

/// Attach all active notes for an org to all its active posts.
///
/// Uses `ON CONFLICT DO NOTHING` for idempotency — safe to call multiple times.
/// Called after note generation completes in crawl/scrape workflows.
pub async fn attach_notes_to_org_posts(
    org_id: OrganizationId,
    pool: &PgPool,
) -> Result<AttachResult> {
    let notes = Note::find_active_for_entity("organization", org_id.into_uuid(), pool).await?;
    let posts = Post::find_by_organization_id(org_id.into_uuid(), pool).await?;

    if notes.is_empty() || posts.is_empty() {
        return Ok(AttachResult {
            notes_count: notes.len() as i32,
            posts_count: posts.len() as i32,
            noteables_created: 0,
        });
    }

    let mut attached = 0;
    for note in &notes {
        for post in &posts {
            match Noteable::create(note.id, "post", post.id.into_uuid(), pool).await {
                Ok(_) => attached += 1,
                Err(e) => {
                    warn!(
                        note_id = %note.id,
                        post_id = %post.id,
                        error = %e,
                        "Failed to attach note to post"
                    );
                }
            }
        }
    }

    info!(
        org_id = %org_id,
        notes = notes.len(),
        posts = posts.len(),
        attached,
        "Attached notes to org posts"
    );

    Ok(AttachResult {
        notes_count: notes.len() as i32,
        posts_count: posts.len() as i32,
        noteables_created: attached,
    })
}

/// Attach all active org notes to a single new post.
///
/// Called during post creation to ensure new posts pick up existing org-level notes.
pub async fn attach_org_notes_to_post(
    org_id: OrganizationId,
    post_id: PostId,
    pool: &PgPool,
) -> Result<i32> {
    let notes = Note::find_active_for_entity("organization", org_id.into_uuid(), pool).await?;

    if notes.is_empty() {
        return Ok(0);
    }

    let mut attached = 0;
    for note in &notes {
        match Noteable::create(note.id, "post", post_id.into_uuid(), pool).await {
            Ok(_) => attached += 1,
            Err(e) => {
                warn!(
                    note_id = %note.id,
                    post_id = %post_id,
                    error = %e,
                    "Failed to attach note to post"
                );
            }
        }
    }

    if attached > 0 {
        info!(
            post_id = %post_id,
            attached,
            "Attached org notes to new post"
        );
    }

    Ok(attached)
}
