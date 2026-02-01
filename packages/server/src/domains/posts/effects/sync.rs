use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use crate::kernel::ServerDeps;
use crate::common::{JobId, WebsiteId};
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Sync Effect - Handles SyncPosts command
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct SyncEffect;

#[async_trait]
impl Effect<PostCommand, ServerDeps> for SyncEffect {
    type Event = PostEvent;

    async fn execute(
        &self,
        cmd: PostCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<PostEvent> {
        match cmd {
            PostCommand::SyncPosts {
                source_id,
                job_id,
                posts,
            } => handle_sync_posts(source_id, job_id, posts, &ctx).await,
            _ => anyhow::bail!("SyncEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler function
// ============================================================================

async fn handle_sync_posts(
    source_id: WebsiteId,
    job_id: JobId,
    posts: Vec<crate::common::ExtractedPost>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        listings_count = posts.len(),
        "Starting database sync for extracted listings"
    );

    let result =
        match super::syncing::sync_extracted_posts(
            source_id,
            posts,
            &ctx.deps().db_pool,
        )
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
                return Ok(PostEvent::SyncFailed {
                    source_id,
                    job_id,
                    reason: format!("Failed to sync posts: {}", e),
                });
            }
        };

    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        "Emitting PostsSynced event"
    );
    Ok(PostEvent::PostsSynced {
        source_id,
        job_id,
        new_count: result.new_count,
        updated_count: result.updated_count,
        unchanged_count: result.unchanged_count,
    })
}
