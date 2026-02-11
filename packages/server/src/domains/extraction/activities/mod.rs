//! Extraction domain actions - business logic functions
//!
//! These actions provide the interface between the API and the extraction library.

use anyhow::{Context, Result};
use extraction::DiscoverConfig;
use tracing::info;

use crate::impl_restate_serde;
use crate::kernel::{HttpIngestor, ServerDeps, ValidatedIngestor};

// =============================================================================
// URL Submission
// =============================================================================

/// Submit a URL for extraction.
///
/// This crawls the URL, stores it in the extraction index, and runs
/// an extraction query to pull out relevant information.
///
/// # Arguments
/// * `url` - The URL to submit
/// * `query` - Optional extraction query (default: "events, services, or opportunities")
/// * `deps` - Server dependencies
///
/// # Returns
/// A list of extractions found at the URL
pub async fn submit_url(
    url: &str,
    query: Option<&str>,
    deps: &ServerDeps,
) -> Result<Vec<extraction::Extraction>> {
    info!(url = %url, "Submitting URL for extraction");

    // Get the extraction service (required)
    let extraction_service = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // Ingest the URL using the new Ingestor pattern
    let ingestor = ValidatedIngestor::new(HttpIngestor::new());
    extraction_service
        .ingest_url(url, &ingestor)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ingest URL: {}", e))?;

    // Extract the site from the URL for filtering
    let site = extract_site(url);
    let default_query = "Extract all volunteer opportunities, services, programs, and events with comprehensive details. \
For each item include: full title, detailed description, contact information \
(phone, email, website), physical location/address, schedule/hours, \
eligibility requirements, and how to sign up or get involved.";
    let extraction_query = query.unwrap_or(default_query);

    // Run extraction on the ingested content
    let extractions = extraction_service
        .extract(extraction_query, Some(&site))
        .await
        .context("Failed to run extraction")?;

    info!(
        url = %url,
        extractions_count = extractions.len(),
        "URL extraction complete"
    );

    Ok(extractions)
}

/// Submit a URL and return a single extraction result.
///
/// Convenience wrapper that returns the first extraction or an empty one.
pub async fn submit_url_one(
    url: &str,
    query: Option<&str>,
    deps: &ServerDeps,
) -> Result<extraction::Extraction> {
    let extractions = submit_url(url, query, deps).await?;
    Ok(extractions
        .into_iter()
        .next()
        .unwrap_or_else(|| extraction::Extraction::new("No content found.".to_string())))
}

// =============================================================================
// Extraction Queries
// =============================================================================

/// Trigger an extraction query.
///
/// Runs an extraction query against the stored content, optionally
/// filtered to a specific site.
///
/// # Arguments
/// * `query` - The extraction query (natural language)
/// * `site` - Optional site filter (e.g., "redcross.org")
/// * `deps` - Server dependencies
///
/// # Returns
/// A list of extractions matching the query
pub async fn trigger_extraction(
    query: &str,
    site: Option<&str>,
    deps: &ServerDeps,
) -> Result<Vec<extraction::Extraction>> {
    info!(query = %query, site = ?site, "Triggering extraction");

    // Get the extraction service (required)
    let extraction_service = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    let extractions = extraction_service
        .extract(query, site)
        .await
        .context("Extraction query failed")?;

    info!(
        query = %query,
        extractions_count = extractions.len(),
        "Extraction query complete"
    );

    Ok(extractions)
}

/// Trigger an extraction and return a single result.
pub async fn trigger_extraction_one(
    query: &str,
    site: Option<&str>,
    deps: &ServerDeps,
) -> Result<extraction::Extraction> {
    let extractions = trigger_extraction(query, site, deps).await?;
    Ok(extractions
        .into_iter()
        .next()
        .unwrap_or_else(|| extraction::Extraction::new("No matching content found.".to_string())))
}

// =============================================================================
// Site Ingestion (Admin)
// =============================================================================

/// Ingest an entire site for extraction.
///
/// This crawls the site (up to configured limits), summarizes pages,
/// and stores them for future extraction queries.
///
/// Admin only.
///
/// # Arguments
/// * `site_url` - The site URL to ingest
/// * `max_pages` - Maximum pages to crawl
/// * `deps` - Server dependencies
pub async fn ingest_site(
    site_url: &str,
    max_pages: Option<i32>,
    deps: &ServerDeps,
) -> Result<IngestSiteResult> {
    info!(site_url = %site_url, max_pages = ?max_pages, "Ingesting site");

    // Get the extraction service (required)
    let extraction_service = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // Use the new Ingestor pattern
    let ingestor = ValidatedIngestor::new(HttpIngestor::new());
    let discover_config =
        DiscoverConfig::new(site_url).with_limit(max_pages.unwrap_or(50) as usize);

    // Run ingestion
    let result = extraction_service
        .ingest(&discover_config, &ingestor)
        .await
        .map_err(|e| anyhow::anyhow!("Site ingestion failed: {}", e))?;

    info!(
        site_url = %site_url,
        pages_crawled = result.pages_crawled,
        pages_summarized = result.pages_summarized,
        "Site ingestion complete"
    );

    Ok(IngestSiteResult {
        site_url: site_url.to_string(),
        pages_crawled: result.pages_crawled as i32,
        pages_summarized: result.pages_summarized as i32,
        pages_skipped: result.pages_skipped as i32,
    })
}

/// Result of site ingestion
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestSiteResult {
    pub site_url: String,
    pub pages_crawled: i32,
    pub pages_summarized: i32,
    pub pages_skipped: i32,
}

impl_restate_serde!(IngestSiteResult);

// =============================================================================
// Helpers
// =============================================================================

/// Extract the site (domain) from a URL.
fn extract_site(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_site() {
        assert_eq!(extract_site("https://example.com/page"), "example.com");
        assert_eq!(
            extract_site("https://www.redcross.org/volunteer"),
            "www.redcross.org"
        );
        assert_eq!(extract_site("invalid"), "invalid");
    }
}
