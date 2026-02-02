//! Crawl website action
//!
//! Execute crawl and return crawled page info.

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{JobId, WebsiteId};
use crate::domains::crawling::events::{CrawledPageInfo, CrawlEvent};
use crate::domains::crawling::models::{PageSnapshot, WebsiteSnapshot};
use crate::domains::website::models::Website;
use crate::kernel::{BaseWebScraper, LinkPriorities, ServerDeps};

/// Build link priorities from static keywords for crawling.
pub fn get_crawl_priorities() -> LinkPriorities {
    LinkPriorities {
        high: HIGH_PRIORITY_KEYWORDS.iter().map(|s| s.to_string()).collect(),
        skip: SKIP_KEYWORDS.iter().map(|s| s.to_string()).collect(),
    }
}

/// High-priority keywords - pages containing these are crawled first
const HIGH_PRIORITY_KEYWORDS: &[&str] = &[
    "services", "programs", "resources", "help", "assistance", "support",
    "volunteer", "donate", "give", "get-involved", "ways-to-help",
    "about", "contact", "location", "hours",
    "food", "housing", "legal", "immigration", "healthcare", "employment", "education", "childcare",
];

/// Skip keywords - pages containing these are not crawled
const SKIP_KEYWORDS: &[&str] = &[
    "login", "signin", "signup", "register", "cart", "checkout", "account", "password", "reset",
    "gallery", "photos", "videos", "downloads", "pdf",
    "privacy", "terms", "cookie", "disclaimer",
    "facebook", "twitter", "instagram", "linkedin", "youtube",
    "search", "sitemap", "rss", "feed", "print", "share",
];

/// Crawl website pages and store snapshots.
///
/// Returns list of crawled page info or failure event.
pub async fn crawl_website_pages(
    website: &Website,
    job_id: JobId,
    web_scraper: &dyn BaseWebScraper,
    deps: &ServerDeps,
) -> Result<Vec<CrawledPageInfo>, CrawlEvent> {
    let website_id = website.id;
    let max_depth = website.max_crawl_depth;
    let max_pages = website.max_pages_per_crawl.unwrap_or(20);
    let delay = website.crawl_rate_limit_seconds;

    let priorities = get_crawl_priorities();

    info!(
        website_id = %website_id,
        url = %website.domain,
        max_depth = %max_depth,
        max_pages = %max_pages,
        "Initiating website crawl"
    );

    let crawl_result = match web_scraper
        .crawl(&website.domain, max_depth, max_pages, delay, Some(&priorities))
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Mark crawl as failed
            let _ = Website::complete_crawl(website_id, "failed", 0, &deps.db_pool).await;
            return Err(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Crawl failed: {}", e),
            });
        }
    };

    info!(
        website_id = %website_id,
        pages_count = crawl_result.pages.len(),
        "Crawl completed, storing page snapshots"
    );

    // Store each crawled page as a snapshot
    let crawled_pages = store_crawled_pages(website_id, crawl_result.pages, &deps.db_pool).await;

    Ok(crawled_pages)
}

/// Store crawled pages as snapshots and link to website.
pub async fn store_crawled_pages(
    website_id: WebsiteId,
    pages: Vec<crate::kernel::CrawledPage>,
    pool: &sqlx::PgPool,
) -> Vec<CrawledPageInfo> {
    let mut crawled_pages: Vec<CrawledPageInfo> = Vec::new();

    for page in pages {
        // Create page snapshot
        let (page_snapshot, _is_new) = match PageSnapshot::upsert(
            pool,
            page.url.clone(),
            page.markdown.clone(),
            Some(page.markdown.clone()),
            "simple_scraper".to_string(),
        )
        .await
        {
            Ok(snapshot) => snapshot,
            Err(e) => {
                warn!(
                    url = %page.url,
                    error = %e,
                    "Failed to store page snapshot, skipping"
                );
                continue;
            }
        };

        // Create website_snapshot entry
        match WebsiteSnapshot::upsert(pool, website_id, page.url.clone(), None).await {
            Ok(website_snapshot) => {
                // Link to page snapshot
                if let Err(e) = website_snapshot.link_snapshot(pool, page_snapshot.id).await {
                    warn!(
                        website_snapshot_id = %website_snapshot.id,
                        error = %e,
                        "Failed to link website_snapshot to page_snapshot"
                    );
                }
            }
            Err(e) => {
                warn!(
                    url = %page.url,
                    error = %e,
                    "Failed to create website_snapshot"
                );
            }
        }

        crawled_pages.push(CrawledPageInfo {
            url: page.url,
            title: page.title,
            snapshot_id: Some(page_snapshot.id),
        });
    }

    crawled_pages
}
