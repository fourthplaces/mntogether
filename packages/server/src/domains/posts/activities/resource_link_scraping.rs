//! Resource link scraping activity
//!
//! Scrapes a submitted resource link URL using the extraction library.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::JobId;
use crate::impl_restate_serde;
use crate::kernel::{FirecrawlIngestor, HttpIngestor, ServerDeps, ValidatedIngestor};

/// Result of scraping a resource link (journaled by Restate between workflow steps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub content: String,
    pub context: Option<String>,
    pub submitter_contact: Option<String>,
}

impl_restate_serde!(ScrapeResult);

/// Scrape a resource link URL using the extraction library.
pub async fn scrape_resource_link(
    job_id: JobId,
    url: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<ScrapeResult> {
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

            Ok(ScrapeResult {
                content: String::new(), // Content is now in extraction_pages
                context,
                submitter_contact,
            })
        }
        Err(e) => {
            tracing::error!(job_id = %job_id, url = %url, error = %e, "Scraping failed");
            Err(anyhow::anyhow!("Web scraping failed: {}", e))
        }
    }
}
