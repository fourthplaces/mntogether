use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::deps::ServerDeps;
use super::listing::extract_domain;
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;
use crate::domains::scraping::models::{PageSnapshot, Website, WebsiteSnapshot};

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
    source_id: WebsiteId,
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
    let source = match Website::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => {
            tracing::info!(
                source_id = %source_id,
                domain = %s.domain,
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

    // Scrape the domain via Firecrawl
    tracing::info!(
        source_id = %source_id,
        domain = %source.domain,
        max_depth = source.max_crawl_depth,
        rate_limit = source.crawl_rate_limit_seconds,
        "Starting domain scrape via Firecrawl"
    );

    let scrape_result = match ctx.deps().web_scraper.scrape(&source.domain).await {
        Ok(r) => {
            tracing::info!(
                source_id = %source_id,
                content_length = r.markdown.len(),
                "Scrape completed successfully"
            );
            r
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                domain = %source.domain,
                error = %e,
                "Scraping failed"
            );
            return Ok(ListingEvent::ScrapeFailed {
                source_id,
                job_id,
                reason: format!("Scraping failed: {}", e),
            });
        }
    };

    // Store scraped content in page_snapshots (with deduplication)
    // Note: Using markdown for html field until we add html to ScrapeResult
    tracing::info!(source_id = %source_id, "Storing page snapshot");
    let (page_snapshot, is_new) = match PageSnapshot::upsert(
        &ctx.deps().db_pool,
        source.domain.clone(),
        scrape_result.markdown.clone(), // Use markdown as html for now
        Some(scrape_result.markdown.clone()),
        "firecrawl".to_string(),
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "Failed to store page snapshot"
            );
            // Continue anyway - we have the content to extract from
            (
                PageSnapshot {
                    id: uuid::Uuid::new_v4(),
                    url: source.domain.clone(),
                    content_hash: vec![],
                    html: scrape_result.markdown.clone(),
                    markdown: Some(scrape_result.markdown.clone()),
                    fetched_via: "firecrawl".to_string(),
                    metadata: serde_json::json!({}),
                    crawled_at: chrono::Utc::now(),
                    listings_extracted_count: Some(0),
                    extraction_completed_at: None,
                    extraction_status: Some("pending".to_string()),
                },
                true,
            )
        }
    };

    if is_new {
        tracing::info!(
            source_id = %source_id,
            page_snapshot_id = %page_snapshot.id,
            "Created new page snapshot"
        );
    } else {
        tracing::info!(
            source_id = %source_id,
            page_snapshot_id = %page_snapshot.id,
            "Reused existing page snapshot (content unchanged)"
        );
    }

    // Create or update website_snapshot for this scrape
    // This creates traceability: website_snapshot -> page_snapshot -> listings
    tracing::info!(
        source_id = %source_id,
        page_url = %source.domain,
        "Creating/updating website_snapshot entry"
    );

    match WebsiteSnapshot::upsert(
        &ctx.deps().db_pool,
        source_id,
        source.domain.clone(),
        None, // No specific submitter for manual admin scrapes
    )
    .await
    {
        Ok(website_snapshot) => {
            tracing::info!(
                website_snapshot_id = %website_snapshot.id,
                page_snapshot_id = %page_snapshot.id,
                "Linking website_snapshot to page_snapshot"
            );
            if let Err(e) = website_snapshot
                .link_snapshot(&ctx.deps().db_pool, page_snapshot.id)
                .await
            {
                tracing::warn!(
                    website_snapshot_id = %website_snapshot.id,
                    error = %e,
                    "Failed to link website_snapshot to page_snapshot"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                source_id = %source_id,
                error = %e,
                "Failed to create website_snapshot, continuing anyway"
            );
        }
    }

    // Update last_scraped_at timestamp
    tracing::info!(source_id = %source_id, "Updating last_scraped_at timestamp");
    if let Err(e) = Website::update_last_scraped(source_id, &ctx.deps().db_pool).await {
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
        page_snapshot_id = %page_snapshot.id,
        organization_name = %extract_domain(&source.domain).unwrap_or_else(|| source.domain.clone()),
        "Scrape completed successfully, emitting SourceScraped event"
    );
    Ok(ListingEvent::SourceScraped {
        source_id,
        job_id,
        organization_name: extract_domain(&source.domain).unwrap_or_else(|| source.domain.clone()),
        content: scrape_result.markdown,
        page_snapshot_id: Some(page_snapshot.id),
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

    // Store scraped content in page_snapshots (with deduplication)
    // Note: Using markdown for html field until we add html to ScrapeResult
    tracing::info!(job_id = %job_id, "Storing page snapshot for resource link");
    let (page_snapshot, is_new) = match PageSnapshot::upsert(
        &ctx.deps().db_pool,
        url.clone(),
        scrape_result.markdown.clone(), // Use markdown as html for now
        Some(scrape_result.markdown.clone()),
        "firecrawl".to_string(),
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(e) => {
            tracing::warn!(
                job_id = %job_id,
                error = %e,
                "Failed to store page snapshot, continuing with extraction"
            );
            // Continue anyway - we have the content to extract from
            (
                PageSnapshot {
                    id: uuid::Uuid::new_v4(),
                    url: url.clone(),
                    content_hash: vec![],
                    html: scrape_result.markdown.clone(),
                    markdown: Some(scrape_result.markdown.clone()),
                    fetched_via: "firecrawl".to_string(),
                    metadata: serde_json::json!({}),
                    crawled_at: chrono::Utc::now(),
                    listings_extracted_count: Some(0),
                    extraction_completed_at: None,
                    extraction_status: Some("pending".to_string()),
                },
                true,
            )
        }
    };

    if is_new {
        tracing::info!(
            job_id = %job_id,
            page_snapshot_id = %page_snapshot.id,
            "Created new page snapshot for resource link"
        );
    } else {
        tracing::info!(
            job_id = %job_id,
            page_snapshot_id = %page_snapshot.id,
            "Reused existing page snapshot (content unchanged)"
        );
    }

    // Return fact event with scraped content
    tracing::info!(
        job_id = %job_id,
        url = %url,
        page_snapshot_id = %page_snapshot.id,
        "Emitting ResourceLinkScraped event"
    );
    Ok(ListingEvent::ResourceLinkScraped {
        job_id,
        url,
        content: scrape_result.markdown,
        context,
        submitter_contact,
        page_snapshot_id: Some(page_snapshot.id),
    })
}
