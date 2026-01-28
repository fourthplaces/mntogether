use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use uuid::Uuid;

use super::ServerDeps;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::source::OrganizationSource;

/// Scraper Effect - Handles ScrapeSource command
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
            OrganizationCommand::ScrapeSource { source_id, .. } => {
                // Get source from database using model layer
                let source = OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool)
                    .await
                    .context("Failed to find source")?;

                // Scrape website using Firecrawl
                let scrape_result = ctx
                    .deps()
                    .firecrawl_client
                    .scrape(&source.source_url)
                    .await
                    .context("Firecrawl scraping failed")?;

                // Update last_scraped_at timestamp
                OrganizationSource::update_last_scraped(source_id, &ctx.deps().db_pool)
                    .await
                    .context("Failed to update last_scraped_at")?;

                // Return fact event
                Ok(OrganizationEvent::SourceScraped {
                    source_id,
                    job_id: Uuid::new_v4(), // job_id from command if needed
                    organization_name: source.organization_name,
                    content: scrape_result.markdown,
                })
            }
            _ => anyhow::bail!("ScraperEffect: Unexpected command"),
        }
    }
}
