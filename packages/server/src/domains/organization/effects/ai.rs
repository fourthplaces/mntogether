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
            OrganizationCommand::ExtractNeedsFromResourceLink {
                job_id,
                url,
                content,
                context,
                submitter_contact,
            } => handle_extract_needs_from_resource_link(job_id, url, content, context, submitter_contact, &ctx).await,
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

async fn handle_extract_needs_from_resource_link(
    job_id: JobId,
    url: String,
    content: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Try to extract organization name from URL (domain)
    let organization_name = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("Unknown Organization")
        .to_string();

    // Add context to the content if provided
    let content_with_context = if let Some(ref ctx_text) = context {
        format!("Context: {}\n\n{}", ctx_text, content)
    } else {
        content
    };

    // Delegate to domain function
    let extracted_needs = match need_extraction::extract_needs(
        ctx.deps().ai.as_ref(),
        &organization_name,
        &content_with_context,
        &url,
    )
    .await
    {
        Ok(needs) => needs,
        Err(e) => {
            return Ok(OrganizationEvent::ResourceLinkScrapeFailed {
                job_id,
                reason: format!("AI extraction failed: {}", e),
            });
        }
    };

    // Return fact event
    Ok(OrganizationEvent::ResourceLinkNeedsExtracted {
        job_id,
        url,
        needs: extracted_needs,
        context,
        submitter_contact,
    })
}
