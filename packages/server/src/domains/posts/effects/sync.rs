//! Sync effect handlers - handle post sync request events
//!
//! These handlers emit events directly and are called from the composite effect.
//!
//! Uses LLM-powered sync to intelligently diff fresh posts against existing DB:
//! - INSERT: New posts that don't exist in DB
//! - UPDATE: Fresh posts that match existing (semantically)
//! - DELETE: DB posts no longer in fresh extraction
//! - MERGE: Consolidate duplicate existing posts

use anyhow::Result;
use seesaw_core::EffectContext;

use crate::common::AppState;
use crate::common::{JobId, WebsiteId};
use crate::domains::posts::actions::llm_sync::llm_sync_posts;
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

// ============================================================================
// Handler action - emits events directly
// ============================================================================

pub async fn handle_sync_posts(
    source_id: WebsiteId,
    job_id: JobId,
    posts: Vec<crate::common::ExtractedPost>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        listings_count = posts.len(),
        "Starting LLM-powered database sync for extracted listings"
    );

    let deps = ctx.deps();
    let result = match llm_sync_posts(source_id, posts, deps.ai.as_ref(), &deps.db_pool).await {
        Ok(r) => {
            tracing::info!(
                source_id = %source_id,
                inserted = r.inserted,
                updated = r.updated,
                deleted = r.deleted,
                merged = r.merged,
                errors_count = r.errors.len(),
                "LLM sync completed successfully"
            );
            r
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "LLM sync failed"
            );
            return Err(anyhow::anyhow!("Failed to sync posts: {}", e));
        }
    };

    // Log any non-fatal errors
    for error in &result.errors {
        tracing::warn!(source_id = %source_id, error = %error, "Sync operation error");
    }

    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        "Emitting PostsSynced event"
    );
    ctx.emit(PostEvent::PostsSynced {
        source_id,
        job_id,
        new_count: result.inserted,
        updated_count: result.updated,
        unchanged_count: 0, // LLM sync doesn't track unchanged
    });
    Ok(())
}
