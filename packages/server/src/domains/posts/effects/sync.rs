use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::{JobId, WebsiteId};
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Sync Effect - Handles SyncListings command
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
            PostCommand::SyncListings {
                source_id,
                job_id,
                listings,
            } => handle_sync_listings(source_id, job_id, listings, &ctx).await,
            _ => anyhow::bail!("SyncEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler function
// ============================================================================

async fn handle_sync_listings(
    source_id: WebsiteId,
    job_id: JobId,
    listings: Vec<crate::common::ExtractedPost>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        listings_count = listings.len(),
        "Starting database sync for extracted listings"
    );

    let result =
        match super::syncing::sync_extracted_listings(source_id, listings, &ctx.deps().db_pool)
            .await
        {
            Ok(r) => {
                tracing::info!(
                    source_id = %source_id,
                    new_count = r.new_count,
                    changed_count = r.changed_count,
                    disappeared_count = r.disappeared_count,
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
                    reason: format!("Failed to sync listings: {}", e),
                });
            }
        };

    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        "Emitting ListingsSynced event"
    );
    Ok(PostEvent::ListingsSynced {
        source_id,
        job_id,
        new_count: result.new_count,
        changed_count: result.changed_count,
        disappeared_count: result.disappeared_count,
    })
}
