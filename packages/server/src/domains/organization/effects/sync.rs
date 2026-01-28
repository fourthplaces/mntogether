use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Sync Effect - Handles SyncNeeds command
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
            } => {
                let result =
                    super::syncing::sync_extracted_needs(source_id, needs, &ctx.deps().db_pool)
                        .await?;

                Ok(OrganizationEvent::NeedsSynced {
                    source_id,
                    job_id,
                    new_count: result.new_count,
                    changed_count: result.changed_count,
                    disappeared_count: result.disappeared_count,
                })
            }
            _ => anyhow::bail!("SyncEffect: Unexpected command"),
        }
    }
}
