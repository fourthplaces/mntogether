//! Scraper cascade handlers
//!
//! These handlers respond to fact events and are called from the composite effect.
//! Entry-point actions live in `actions/`, not here.
//!
//! This module now uses the extraction library for scraping.

use anyhow::Result;

use crate::common::JobId;
use crate::domains::posts::events::PostEvent;
use crate::kernel::{FirecrawlIngestor, HttpIngestor, ServerDeps, ValidatedIngestor};

/// Cascade handler: WebsiteCreatedFromLink → scrape resource link
///
/// Uses the extraction library to ingest the URL, returns ResourceLinkScraped event.
pub async fn handle_scrape_resource_link(
    job_id: JobId,
    url: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<PostEvent> {
    tracing::info!(
        job_id = %job_id,
        url = %url,
        context = ?context,
        "Starting resource link scrape via extraction library"
    );

    // Get extraction service (required)
    let extraction = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // Use extraction library to ingest the single URL
    let urls = vec![url.clone()];
    let result = match FirecrawlIngestor::from_env() {
        Ok(firecrawl) => {
            let ingestor = ValidatedIngestor::new(firecrawl);
            extraction.ingest_urls(&urls, &ingestor).await
        }
        Err(_) => {
            let http = HttpIngestor::new();
            let ingestor = ValidatedIngestor::new(http);
            extraction.ingest_urls(&urls, &ingestor).await
        }
    };

    match result {
        Ok(ingest_result) => {
            tracing::info!(
                job_id = %job_id,
                url = %url,
                pages_summarized = ingest_result.pages_summarized,
                "Resource link ingested via extraction library"
            );

            Ok(PostEvent::ResourceLinkScraped {
                job_id,
                url,
                content: String::new(), // Content is now in extraction_pages
                context,
                submitter_contact,
                page_snapshot_id: None, // No longer using page_snapshots
            })
        }
        Err(e) => {
            tracing::error!(job_id = %job_id, url = %url, error = %e, "Scraping failed");
            Err(anyhow::anyhow!("Web scraping failed: {}", e))
        }
    }
}
