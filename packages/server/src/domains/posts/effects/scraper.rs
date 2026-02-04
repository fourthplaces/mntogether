//! Scraper cascade handlers
//!
//! These handlers respond to fact events and are called from the composite effect.
//! Entry-point actions live in `actions/`, not here.
//!
//! This module now uses the extraction library for scraping.

use anyhow::Result;
use seesaw_core::EffectContext;

use crate::common::AppState;
use crate::common::JobId;
use crate::domains::posts::events::PostEvent;
use crate::kernel::{FirecrawlIngestor, HttpIngestor, ServerDeps, ValidatedIngestor};

/// Cascade handler: WebsiteCreatedFromLink → scrape resource link
///
/// Uses the extraction library to ingest the URL, then emits ResourceLinkScraped.
pub async fn handle_scrape_resource_link(
    job_id: JobId,
    url: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    tracing::info!(
        job_id = %job_id,
        url = %url,
        context = ?context,
        "Starting resource link scrape via extraction library"
    );

    // Use extraction library to ingest the single URL
    let urls = vec![url.clone()];
    let result = match FirecrawlIngestor::from_env() {
        Ok(firecrawl) => {
            let ingestor = ValidatedIngestor::new(firecrawl);
            ctx.deps().extraction.ingest_urls(&urls, &ingestor).await
        }
        Err(_) => {
            let http = HttpIngestor::new();
            let ingestor = ValidatedIngestor::new(http);
            ctx.deps().extraction.ingest_urls(&urls, &ingestor).await
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

            ctx.emit(PostEvent::ResourceLinkScraped {
                job_id,
                url,
                content: String::new(), // Content is now in extraction_pages
                context,
                submitter_contact,
                page_snapshot_id: None, // No longer using page_snapshots
            });
        }
        Err(e) => {
            tracing::error!(job_id = %job_id, url = %url, error = %e, "Scraping failed");
            return Err(anyhow::anyhow!("Web scraping failed: {}", e));
        }
    }

    Ok(())
}
