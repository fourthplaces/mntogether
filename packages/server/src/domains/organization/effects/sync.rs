use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::{JobId, SourceId};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Sync Effect - Handles SyncNeeds command
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct SyncEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for SyncEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::SyncNeeds {
                source_id,
                job_id,
                needs,
            } => handle_sync_needs(source_id, job_id, needs, &ctx).await,
            _ => anyhow::bail!("SyncEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler function
// ============================================================================

async fn handle_sync_needs(
    source_id: SourceId,
    job_id: JobId,
    needs: Vec<crate::common::ExtractedNeed>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        needs_count = needs.len(),
        "Starting database sync for extracted needs"
    );

    let result =
        match super::syncing::sync_extracted_needs(source_id, needs, &ctx.deps().db_pool).await {
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
                return Ok(OrganizationEvent::SyncFailed {
                    source_id,
                    job_id,
                    reason: format!("Failed to sync needs: {}", e),
                });
            }
        };

    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        "Emitting NeedsSynced event"
    );
    Ok(OrganizationEvent::NeedsSynced {
        source_id,
        job_id,
        new_count: result.new_count,
        changed_count: result.changed_count,
        disappeared_count: result.disappeared_count,
    })
}
