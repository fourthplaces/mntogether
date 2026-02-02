//! Build pages for summarization
//!
//! Loads page snapshots and builds PageToSummarize list.

use std::collections::HashMap;

use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::common::WebsiteId;
use crate::domains::crawling::effects::extraction::{hash_content, PageToSummarize};
use crate::domains::crawling::events::CrawledPageInfo;
use crate::domains::crawling::models::{PageSnapshot, WebsiteSnapshot};
use crate::domains::website::models::Website;

/// Load page snapshots and build PageToSummarize list.
///
/// Returns a tuple of (pages_to_summarize, url_to_snapshot_id_map).
pub async fn build_pages_to_summarize(
    pages: &[CrawledPageInfo],
    pool: &PgPool,
) -> Result<(Vec<PageToSummarize>, HashMap<String, Uuid>)> {
    let mut pages_to_summarize: Vec<PageToSummarize> = Vec::new();
    let mut snapshot_map: HashMap<String, Uuid> = HashMap::new();

    for page in pages {
        let Some(snapshot_id) = page.snapshot_id else {
            warn!(url = %page.url, "No snapshot ID for page, skipping");
            continue;
        };

        let snapshot = match PageSnapshot::find_by_id(pool, snapshot_id).await {
            Ok(s) => s,
            Err(e) => {
                warn!(snapshot_id = %snapshot_id, error = %e, "Failed to load snapshot");
                continue;
            }
        };

        let raw_content = snapshot.markdown.unwrap_or(snapshot.html);
        let content_hash = hash_content(&raw_content);

        snapshot_map.insert(page.url.clone(), snapshot_id);
        pages_to_summarize.push(PageToSummarize {
            snapshot_id,
            url: page.url.clone(),
            raw_content,
            content_hash,
        });
    }

    Ok((pages_to_summarize, snapshot_map))
}

/// Build a single PageToSummarize from a PageSnapshot.
pub fn build_page_to_summarize_from_snapshot(
    snapshot: &PageSnapshot,
    url: String,
) -> PageToSummarize {
    let raw_content = snapshot
        .markdown
        .clone()
        .unwrap_or_else(|| snapshot.html.clone());
    let content_hash = hash_content(&raw_content);

    PageToSummarize {
        snapshot_id: snapshot.id,
        url,
        raw_content,
        content_hash,
    }
}

/// Context for a single page including its snapshot and associated website.
pub struct SinglePageContext {
    pub page_snapshot: PageSnapshot,
    pub website: Website,
    pub website_id: WebsiteId,
}

/// Fetch page context (snapshot + website) for single-page operations.
///
/// Returns None if the page snapshot or associated website cannot be found.
pub async fn fetch_single_page_context(
    page_snapshot_id: Uuid,
    pool: &PgPool,
) -> Option<SinglePageContext> {
    let page_snapshot = PageSnapshot::find_by_id(pool, page_snapshot_id).await.ok()?;

    let website_snapshot = WebsiteSnapshot::find_by_page_snapshot_id(pool, page_snapshot_id)
        .await
        .ok()
        .flatten()?;

    let website_id = website_snapshot.get_website_id();
    let website = Website::find_by_id(website_id, pool).await.ok()?;

    Some(SinglePageContext {
        page_snapshot,
        website,
        website_id,
    })
}
