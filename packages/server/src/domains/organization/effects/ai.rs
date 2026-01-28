use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::{need_extraction, ServerDeps};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::source::OrganizationSource;

/// AI Effect - Handles ExtractNeeds command
///
/// This is a thin orchestrator that delegates to domain functions.
pub struct AIEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for AIEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::ExtractNeeds {
                source_id,
                job_id,
                organization_name,
                content,
            } => {
                // Get source for URL info
                let source = OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool)
                    .await
                    .context("Failed to find source")?;

                // Delegate to domain function
                let extracted_needs = need_extraction::extract_needs(
                    ctx.deps().ai.as_ref(),
                    &organization_name,
                    &content,
                    &source.source_url,
                )
                .await
                .context("AI extraction failed")?;

                // Return fact event
                Ok(OrganizationEvent::NeedsExtracted {
                    source_id,
                    job_id,
                    needs: extracted_needs,
                })
            }
            _ => anyhow::bail!("AIEffect: Unexpected command"),
        }
    }
}
