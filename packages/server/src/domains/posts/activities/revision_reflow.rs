//! Revision handling for Root Signal ingest (spec §12.1, TODO §1.4).
//!
//! When an ingest submission carries `editorial.revision_of_post_id`:
//!
//!   1. Archive the prior post (`status = 'archived'`). Its `revision_of_post_id`
//!      chain is preserved so `/admin/posts/{id}` can render history.
//!   2. Find every active edition the prior post was slotted into and run
//!      `generate_edition` on each. The layout engine picks up the revised
//!      post (which now sits where the old one used to) and fills the slot.
//!      Editors see the updated layout next time they open the edition.
//!
//! We don't attempt surgical slot-swap — full regeneration is the only
//! guaranteed-correct path when the revised post has different weight /
//! post_type / tags than the original. Regeneration is idempotent and the
//! layout engine is fast enough that this is fine per-edition.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;
use crate::domains::editions::activities::edition_ops;
use crate::domains::posts::models::Post;
use crate::kernel::ServerDeps;

pub struct ReflowResult {
    pub archived_post_id: Uuid,
    pub reflowed_edition_ids: Vec<Uuid>,
}

/// Archive the prior post and reflow every active edition that contained it.
/// Called by the ingest orchestrator *after* the new post row is inserted,
/// so the regenerated layout sees the revised post.
pub async fn archive_and_reflow(
    prior_post_id: Uuid,
    deps: &ServerDeps,
) -> Result<ReflowResult> {
    let pool = &deps.db_pool;
    let prior = PostId::from_uuid(prior_post_id);

    // 1. Archive prior post.
    Post::update_status(prior, "archived", pool).await?;

    // 2. Find every non-published edition that slotted the prior post.
    //    Published editions are frozen in place — revisions arriving after
    //    publish are editorial-only concerns (reflow a published edition
    //    would rewrite the paper of record).
    let edition_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT e.id
        FROM editions e
        JOIN edition_rows er ON er.edition_id = e.id
        JOIN edition_slots es ON es.edition_row_id = er.id
        WHERE es.post_id = $1
          AND e.status IN ('draft', 'in_review', 'approved')
        "#,
    )
    .bind(prior_post_id)
    .fetch_all(pool)
    .await?;

    let mut reflowed = Vec::with_capacity(edition_ids.len());
    for eid in edition_ids {
        match edition_ops::generate_edition(eid, deps).await {
            Ok(_) => reflowed.push(eid),
            Err(err) => {
                // Log and continue — a bad single edition shouldn't block the
                // ingest from completing. The prior post is already archived
                // and the new one is saved.
                tracing::warn!(
                    edition_id = %eid,
                    prior_post_id = %prior_post_id,
                    error = %err,
                    "revision reflow failed for edition; continuing"
                );
            }
        }
    }

    Ok(ReflowResult {
        archived_post_id: prior_post_id,
        reflowed_edition_ids: reflowed,
    })
}
