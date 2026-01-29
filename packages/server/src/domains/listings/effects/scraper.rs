use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{JobId, MemberId, SourceId};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;
use crate::domains::organization::models::source::OrganizationSource;

/// Scraper Effect - Handles ScrapeSource command
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct ScraperEffect;

#[async_trait]
impl Effect<ListingCommand, ServerDeps> for ScraperEffect {
    type Event = ListingEvent;

    async fn execute(
        &self,
        cmd: ListingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ListingEvent> {
        match cmd {
            ListingCommand::ScrapeSource {
                source_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_scrape_source(source_id, job_id, requested_by, is_admin, &ctx).await,
            ListingCommand::ScrapeResourceLink {
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
    _is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Starting scrape source handler"
    );

    // Authorization check - only admins can scrape sources
    if let Err(auth_err) = Actor::new(requested_by, _is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        tracing::warn!(
            source_id = %source_id,
            requested_by = %requested_by,
            error = %auth_err,
            "Authorization denied"
        );
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ScrapeSource".to_string(),
            reason: auth_err.to_string(),
        });
    }

    tracing::info!(source_id = %source_id, "Authorization passed, fetching source from database");

    // Get source from database using model layer
    let source = match OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => {
            tracing::info!(
                source_id = %source_id,
                url = %s.source_url,
                org = %s.organization_name,
                "Source found, preparing to scrape"
            );
            s
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "Failed to find source in database"
            );
            return Ok(ListingEvent::ScrapeFailed {
                source_id,
                job_id,
                reason: format!("Failed to find source: {}", e),
            });
        }
    };

    // Check if specific URLs are configured, otherwise crawl the whole site
    let urls_to_scrape: Vec<String> = if let Some(scrape_urls_json) = &source.scrape_urls {
        // Parse scrape_urls JSON array
        match serde_json::from_value::<Vec<String>>(scrape_urls_json.clone()) {
            Ok(urls) if !urls.is_empty() => {
                tracing::info!(
                    source_id = %source_id,
                    url_count = urls.len(),
                    "Using specific scrape URLs instead of crawling"
                );
                urls
            }
            _ => {
                // If parsing failed or empty, fall back to crawling
                tracing::info!(
                    source_id = %source_id,
                    url = %source.source_url,
                    "No specific URLs configured, crawling site"
                );
                vec![source.source_url.clone()]
            }
        }
    } else {
        // No scrape_urls configured, crawl the whole site
        tracing::info!(
            source_id = %source_id,
            url = %source.source_url,
            "No specific URLs configured, crawling site"
        );
        vec![source.source_url.clone()]
    };

    // If multiple URLs, scrape each individually and combine
    // If single URL, it will crawl the site (as before)
    let scrape_result = if urls_to_scrape.len() > 1 {
        tracing::info!(
            source_id = %source_id,
            url_count = urls_to_scrape.len(),
            "Scraping multiple specific URLs"
        );

        let mut combined_markdown = String::new();
        for (idx, url) in urls_to_scrape.iter().enumerate() {
            tracing::info!(
                source_id = %source_id,
                url = %url,
                index = idx + 1,
                total = urls_to_scrape.len(),
                "Scraping URL"
            );

            match ctx.deps().web_scraper.scrape(url).await {
                Ok(result) => {
                    combined_markdown.push_str(&format!(
                        "\n\n--- Source {}/{}: {} ---\n\n{}",
                        idx + 1,
                        urls_to_scrape.len(),
                        url,
                        result.markdown
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        source_id = %source_id,
                        url = %url,
                        error = %e,
                        "Failed to scrape URL, continuing with others"
                    );
                }
            }
        }

        crate::kernel::ScrapeResult {
            url: source.source_url.clone(),
            markdown: combined_markdown,
            title: Some(source.organization_name.clone()),
        }
    } else {
        // Single URL - use normal crawling behavior
        let url = &urls_to_scrape[0];
        tracing::info!(
            source_id = %source_id,
            url = %url,
            "Starting web scrape/crawl via Firecrawl"
        );

        match ctx.deps().web_scraper.scrape(url).await {
            Ok(r) => {
                tracing::info!(
                    source_id = %source_id,
                    content_length = r.markdown.len(),
                    "Web scrape completed successfully"
                );
                r
            }
            Err(e) => {
                tracing::error!(
                    source_id = %source_id,
                    url = %url,
                    error = %e,
                    "Web scraping failed"
                );
                return Ok(ListingEvent::ScrapeFailed {
                    source_id,
                    job_id,
                    reason: format!("Web scraping failed: {}", e),
                });
            }
        }
    };

    // Update last_scraped_at timestamp
    tracing::info!(source_id = %source_id, "Updating last_scraped_at timestamp");
    if let Err(e) = OrganizationSource::update_last_scraped(source_id, &ctx.deps().db_pool).await {
        // Log warning but don't fail the scrape - this is non-critical
        tracing::warn!(
            source_id = %source_id,
            error = %e,
            "Failed to update last_scraped_at timestamp"
        );
    }

    // Return fact event
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        organization_name = %source.organization_name,
        "Scrape completed successfully, emitting SourceScraped event"
    );
    Ok(ListingEvent::SourceScraped {
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
) -> Result<ListingEvent> {
    tracing::info!(
        job_id = %job_id,
        url = %url,
        context = ?context,
        "Starting resource link scrape (public submission)"
    );

    // Public endpoint - no authorization needed

    // Scrape the URL using web scraper
    let scrape_result = match ctx.deps().web_scraper.scrape(&url).await {
        Ok(r) => {
            tracing::info!(
                job_id = %job_id,
                content_length = r.markdown.len(),
                "Resource link scrape completed successfully"
            );
            r
        }
        Err(e) => {
            tracing::error!(
                job_id = %job_id,
                url = %url,
                error = %e,
                "Resource link web scraping failed"
            );
            return Ok(ListingEvent::ResourceLinkScrapeFailed {
                job_id,
                reason: format!("Web scraping failed: {}", e),
            });
        }
    };

    // Return fact event with scraped content
    tracing::info!(
        job_id = %job_id,
        url = %url,
        "Emitting ResourceLinkScraped event"
    );
    Ok(ListingEvent::ResourceLinkScraped {
        job_id,
        url,
        content: scrape_result.markdown,
        context,
        submitter_contact,
    })
}
