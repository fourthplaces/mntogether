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
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        organization_name = %organization_name,
        content_length = content.len(),
        "Starting AI need extraction"
    );

    // Get source for URL info
    let source = match OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => {
            tracing::info!(source_id = %source_id, url = %s.source_url, "Source found for extraction");
            s
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "Failed to find source for extraction"
            );
            return Ok(OrganizationEvent::ExtractFailed {
                source_id,
                job_id,
                reason: format!("Failed to find source: {}", e),
            });
        }
    };

    tracing::info!(
        source_id = %source_id,
        url = %source.source_url,
        "Calling AI service to extract needs"
    );

    // Delegate to domain function
    let extracted_needs = match need_extraction::extract_needs(
        ctx.deps().ai.as_ref(),
        &organization_name,
        &content,
        &source.source_url,
    )
    .await
    {
        Ok(needs) => {
            tracing::info!(
                source_id = %source_id,
                needs_count = needs.len(),
                "AI extraction completed successfully"
            );
            needs
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "AI extraction failed"
            );
            return Ok(OrganizationEvent::ExtractFailed {
                source_id,
                job_id,
                reason: format!("AI extraction failed: {}", e),
            });
        }
    };

    // Return fact event
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        needs_count = extracted_needs.len(),
        "Emitting NeedsExtracted event"
    );
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
    tracing::info!(
        job_id = %job_id,
        url = %url,
        content_length = content.len(),
        has_context = context.is_some(),
        "Starting AI need extraction from resource link"
    );

    // Try to extract organization name from URL (domain)
    let organization_name = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("Unknown Organization")
        .to_string();

    tracing::info!(
        job_id = %job_id,
        organization_name = %organization_name,
        "Extracted organization name from URL"
    );

    // Add context to the content if provided
    let content_with_context = if let Some(ref ctx_text) = context {
        tracing::info!(job_id = %job_id, "Adding user context to content");
        format!("Context: {}\n\n{}", ctx_text, content)
    } else {
        content
    };

    tracing::info!(job_id = %job_id, "Calling AI service to extract needs from resource link");

    // Delegate to domain function
    let extracted_needs = match need_extraction::extract_needs(
        ctx.deps().ai.as_ref(),
        &organization_name,
        &content_with_context,
        &url,
    )
    .await
    {
        Ok(needs) => {
            tracing::info!(
                job_id = %job_id,
                needs_count = needs.len(),
                "AI extraction from resource link completed successfully"
            );
            needs
        }
        Err(e) => {
            tracing::error!(
                job_id = %job_id,
                url = %url,
                error = %e,
                "AI extraction from resource link failed"
            );
            return Ok(OrganizationEvent::ResourceLinkScrapeFailed {
                job_id,
                reason: format!("AI extraction failed: {}", e),
            });
        }
    };

    // Return fact event
    tracing::info!(
        job_id = %job_id,
        url = %url,
        needs_count = extracted_needs.len(),
        "Emitting ResourceLinkNeedsExtracted event"
    );
    Ok(OrganizationEvent::ResourceLinkNeedsExtracted {
        job_id,
        url,
        needs: extracted_needs,
        context,
        submitter_contact,
    })
}
