use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::{need_extraction, ServerDeps};
use crate::common::{JobId, SourceId};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::source::OrganizationSource;

/// AI Effect - Handles ExtractNeeds command
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
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
            } => handle_extract_needs(source_id, job_id, organization_name, content, &ctx).await,
            _ => anyhow::bail!("AIEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler function
// ============================================================================

async fn handle_extract_needs(
    source_id: SourceId,
    job_id: JobId,
    organization_name: String,
    content: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Get source for URL info
    let source = match OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(OrganizationEvent::ExtractFailed {
                source_id,
                job_id,
                reason: format!("Failed to find source: {}", e),
            });
        }
    };

    // Delegate to domain function
    let extracted_needs = match need_extraction::extract_needs(
        ctx.deps().ai.as_ref(),
        &organization_name,
        &content,
        &source.source_url,
    )
    .await
    {
        Ok(needs) => needs,
        Err(e) => {
            return Ok(OrganizationEvent::ExtractFailed {
                source_id,
                job_id,
                reason: format!("AI extraction failed: {}", e),
            });
        }
    };

    // Return fact event
    Ok(OrganizationEvent::NeedsExtracted {
        source_id,
        job_id,
        needs: extracted_needs,
    })
}
