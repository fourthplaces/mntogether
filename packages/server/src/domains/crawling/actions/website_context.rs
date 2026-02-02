//! Website context helpers for crawling workflows
//!
//! Consolidates common website fetch + validation patterns.

use sqlx::PgPool;

use crate::common::WebsiteId;
use crate::domains::crawling::events::CrawledPageInfo;
use crate::domains::crawling::models::WebsiteSnapshot;
use crate::domains::website::models::Website;

/// Fetch an approved website, returning None if not found or not approved.
pub async fn fetch_approved_website(
    website_id: WebsiteId,
    pool: &PgPool,
) -> Option<Website> {
    Website::find_by_id(website_id, pool)
        .await
        .ok()
        .filter(|w| w.status == "approved")
}

/// Fetch website snapshots and convert to CrawledPageInfo list.
///
/// Returns empty vec if no snapshots found.
pub async fn fetch_snapshots_as_crawled_pages(
    website_id: WebsiteId,
    pool: &PgPool,
) -> Vec<CrawledPageInfo> {
    WebsiteSnapshot::find_by_website(pool, website_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| s.page_snapshot_id.map(|ps_id| CrawledPageInfo {
            url: s.page_url,
            title: None,
            snapshot_id: Some(ps_id),
        }))
        .collect()
}
