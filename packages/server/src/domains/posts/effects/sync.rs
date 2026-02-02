//! Sync effect handlers - handle post sync request events
//!
//! These handlers emit events directly and are called from the composite effect.

use anyhow::Result;
use seesaw_core::EffectContext;

use crate::common::AppState;
use crate::common::{JobId, WebsiteId};
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
        "Starting database sync for extracted listings"
    );

    let result = match super::syncing::sync_extracted_posts(source_id, posts, &ctx.deps().db_pool)
        .await
    {
        Ok(r) => {
            tracing::info!(
                source_id = %source_id,
                new_count = r.new_count,
                updated_count = r.updated_count,
                unchanged_count = r.unchanged_count,
                "Database sync completed successfully"
            );
            r
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "Database sync failed"
            );
            ctx.emit(PostEvent::SyncFailed {
                source_id,
                job_id,
                reason: format!("Failed to sync posts: {}", e),
            });
            return Ok(());
        }
    };

    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        "Emitting PostsSynced event"
    );
    ctx.emit(PostEvent::PostsSynced {
        source_id,
        job_id,
        new_count: result.new_count,
        updated_count: result.updated_count,
        unchanged_count: result.unchanged_count,
    });
    Ok(())
}
