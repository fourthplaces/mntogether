//! Full website crawl activity
//!
//! High-level activity that orchestrates the entire crawl pipeline.

use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use crate::common::WebsiteId;
use crate::domains::crawling::workflows::CrawlWebsiteResult;
use crate::kernel::ServerDeps;

/// Crawl a website end-to-end: ingest → extract → investigate → sync
///
/// This is a high-level orchestration activity that performs all crawl steps.
/// Returns simple result data for the workflow.
pub async fn crawl_website_full(
    website_id: WebsiteId,
    visitor_id: Uuid,
    use_firecrawl: bool,
    deps: &ServerDeps,
) -> Result<CrawlWebsiteResult> {
    info!(
        website_id = %website_id,
        visitor_id = %visitor_id,
        use_firecrawl = use_firecrawl,
        "Starting full website crawl"
    );

    // Step 1: Ingest website pages
    let ingest_result = super::ingest_website(
        website_id.into_uuid(),
        visitor_id,
        use_firecrawl,
        true, // Authorization checked at workflow layer
        deps,
    )
    .await?;

    info!(
        website_id = %website_id,
        pages_crawled = ingest_result.pages_crawled,
        "Website ingested successfully"
    );

    // For now, return simple result
    // TODO: Add narrative extraction, investigation, and sync steps
    Ok(CrawlWebsiteResult {
        website_id: website_id.into_uuid(),
        posts_synced: 0, // Will be populated when we add extraction steps
        status: "ingested".to_string(),
    })
}
