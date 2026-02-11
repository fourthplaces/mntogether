//! Note attachment activities.
//!
//! Links active org-level notes to semantically relevant posts using pgvector
//! cosine similarity. Warn-severity notes always attach (org-wide safety alerts).

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::OrganizationId;
use crate::domains::notes::models::{Note, Noteable};
use crate::domains::posts::models::Post;
use crate::kernel::ServerDeps;

/// Similarity threshold for attaching notes to posts.
/// Same threshold used by post semantic search (proven baseline).
const SIMILARITY_THRESHOLD: f64 = 0.3;

pub struct AttachResult {
    pub notes_count: i32,
    pub posts_count: i32,
    pub noteables_created: i32,
}

/// Attach all active notes for an org to semantically relevant posts.
///
/// - Warn-severity notes attach to ALL posts (safety alerts).
/// - Other notes only attach to posts above the similarity threshold.
/// - Posts missing embeddings get them generated on the fly.
///
/// Uses `ON CONFLICT DO NOTHING` for idempotency — safe to call multiple times.
/// Called after note generation completes in crawl/scrape workflows.
pub async fn attach_notes_to_org_posts(
    org_id: OrganizationId,
    deps: &ServerDeps,
) -> Result<AttachResult> {
    let pool = &deps.db_pool;
    let notes = Note::find_active_for_entity("organization", org_id.into_uuid(), pool).await?;
    let posts = Post::find_by_organization_id(org_id.into_uuid(), pool).await?;

    if notes.is_empty() || posts.is_empty() {
        return Ok(AttachResult {
            notes_count: notes.len() as i32,
            posts_count: posts.len() as i32,
            noteables_created: 0,
        });
    }

    // Backfill embeddings for posts that don't have them
    for post in &posts {
        if post.embedding.is_none() {
            let text = post.get_embedding_text();
            match deps.embedding_service.generate(&text).await {
                Ok(emb) => {
                    if let Err(e) = Post::update_embedding(post.id, &emb, pool).await {
                        warn!(post_id = %post.id, error = %e, "Failed to backfill post embedding");
                    }
                }
                Err(e) => {
                    warn!(post_id = %post.id, error = %e, "Failed to generate post embedding");
                }
            }
        }
    }

    let mut attached = 0;

    for note in &notes {
        if note.severity == "urgent" {
            // Urgent-severity: attach to ALL posts (org-wide safety alerts)
            for post in &posts {
                match Noteable::create(note.id, "post", post.id.into_uuid(), pool).await {
                    Ok(_) => attached += 1,
                    Err(e) => {
                        warn!(note_id = %note.id, post_id = %post.id, error = %e, "Failed to attach urgent note");
                    }
                }
            }
        } else if let Some(ref note_embedding) = note.embedding {
            // Info/notice notes: use similarity matching
            let note_vec: Vec<f32> = note_embedding.to_vec();
            match find_similar_post_ids(org_id, &note_vec, SIMILARITY_THRESHOLD, pool).await {
                Ok(post_ids) => {
                    for post_id in post_ids {
                        match Noteable::create(note.id, "post", post_id, pool).await {
                            Ok(_) => attached += 1,
                            Err(e) => {
                                warn!(note_id = %note.id, post_id = %post_id, error = %e, "Failed to attach note");
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(note_id = %note.id, error = %e, "Similarity search failed, skipping note");
                }
            }
        } else {
            // Note without embedding — shouldn't happen but be defensive
            warn!(note_id = %note.id, "Note has no embedding, skipping similarity match");
        }
    }

    info!(
        org_id = %org_id,
        notes = notes.len(),
        posts = posts.len(),
        attached,
        "Attached notes to org posts (similarity-based)"
    );

    Ok(AttachResult {
        notes_count: notes.len() as i32,
        posts_count: posts.len() as i32,
        noteables_created: attached,
    })
}

/// Find post IDs for an org whose embeddings are similar to the given note embedding.
async fn find_similar_post_ids(
    org_id: OrganizationId,
    note_embedding: &[f32],
    threshold: f64,
    pool: &sqlx::PgPool,
) -> Result<Vec<Uuid>> {
    use pgvector::Vector;

    let vector = Vector::from(note_embedding.to_vec());

    let rows = sqlx::query_as::<_, (Uuid,)>(
        r#"
        SELECT p.id
        FROM posts p
        JOIN post_sources ps ON ps.post_id = p.id
        JOIN sources s ON ps.source_id = s.id
        WHERE s.organization_id = $1
          AND p.status = 'active'
          AND p.deleted_at IS NULL
          AND p.revision_of_post_id IS NULL
          AND p.translation_of_id IS NULL
          AND p.embedding IS NOT NULL
          AND (1 - (p.embedding <=> $2)) > $3
        "#,
    )
    .bind(org_id.into_uuid())
    .bind(vector)
    .bind(threshold)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}
