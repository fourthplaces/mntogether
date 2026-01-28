use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::{JobId, MemberId, SourceId};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::source::OrganizationSource;

/// Scraper Effect - Handles ScrapeSource command
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct ScraperEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for ScraperEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::ScrapeSource {
                source_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_scrape_source(source_id, job_id, requested_by, is_admin, &ctx).await,
            OrganizationCommand::ScrapeResourceLink {
                job_id,
                url,
                context,
                submitter_contact,
            } => handle_scrape_resource_link(job_id, url, context, submitter_contact, &ctx).await,
            _ => anyhow::bail!("ScraperEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler function
// ============================================================================

async fn handle_scrape_source(
    source_id: SourceId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can scrape sources
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ScrapeSource".to_string(),
            reason: "Only administrators can scrape organization sources".to_string(),
        });
    }

    // Get source from database using model layer
    let source = match OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(OrganizationEvent::ScrapeFailed {
                source_id,
                job_id,
                reason: format!("Failed to find source: {}", e),
            });
        }
    };

    // Scrape website using web scraper
    let scrape_result = match ctx.deps().web_scraper.scrape(&source.source_url).await {
        Ok(r) => r,
        Err(e) => {
            return Ok(OrganizationEvent::ScrapeFailed {
                source_id,
                job_id,
                reason: format!("Web scraping failed: {}", e),
            });
        }
    };

    // Update last_scraped_at timestamp
    if let Err(e) = OrganizationSource::update_last_scraped(source_id, &ctx.deps().db_pool).await {
        // Log warning but don't fail the scrape - this is non-critical
        tracing::warn!(
            source_id = %source_id,
            error = %e,
            "Failed to update last_scraped_at timestamp"
        );
    }

    // Return fact event
    Ok(OrganizationEvent::SourceScraped {
        source_id,
        job_id,
        organization_name: source.organization_name,
        content: scrape_result.markdown,
    })
}

async fn handle_scrape_resource_link(
    job_id: JobId,
    url: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Public endpoint - no authorization needed

    // Scrape the URL using web scraper
    let scrape_result = match ctx.deps().web_scraper.scrape(&url).await {
        Ok(r) => r,
        Err(e) => {
            return Ok(OrganizationEvent::ResourceLinkScrapeFailed {
                job_id,
                reason: format!("Web scraping failed: {}", e),
            });
        }
    };

    // Return fact event with scraped content
    Ok(OrganizationEvent::ResourceLinkScraped {
        job_id,
        url,
        content: scrape_result.markdown,
        context,
        submitter_contact,
    })
}
