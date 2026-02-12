//! Full website crawl activity
//!
//! High-level activity that orchestrates the entire crawl pipeline.

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{MemberId, WebsiteId};
use crate::domains::crawling::restate::CrawlWebsiteResult;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Crawl a website end-to-end: ingest → extract → investigate → sync
///
/// This is a high-level orchestration activity that performs all crawl steps.
/// Authorization is checked at the workflow layer via require_admin.
/// Returns simple result data for the workflow.
pub async fn crawl_website_full(
    website_id: WebsiteId,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<CrawlWebsiteResult> {
    info!(
        website_id = %website_id,
        requested_by = %requested_by,
        "Starting full website crawl"
    );

    // Step 1: Ingest website pages
    let ingest_result = super::ingest_website(
        website_id.into_uuid(),
        requested_by.into_uuid(),
        true, // Authorization checked at workflow layer
        deps,
    )
    .await?;

    info!(
        website_id = %website_id,
        pages_crawled = ingest_result.pages_crawled,
        "Website ingested successfully"
    );

    // Step 2: Auto-create organization from crawled pages (best-effort)
    let website = Website::find_by_id(website_id, &deps.db_pool).await?;
    if website.organization_id.is_none() {
        match super::extract_and_create_organization(website_id, deps).await {
            Ok(org_id) => {
                info!(website_id = %website_id, org_id = %org_id, "Organization auto-created");
            }
            Err(e) => {
                warn!(website_id = %website_id, error = %e, "Org extraction failed (non-fatal)");
            }
        }
    }

    // TODO: Add narrative extraction, investigation, and sync steps
    Ok(CrawlWebsiteResult {
        website_id: website_id.into_uuid(),
        posts_synced: 0, // Will be populated when we add extraction steps
        status: "ingested".to_string(),
    })
}
