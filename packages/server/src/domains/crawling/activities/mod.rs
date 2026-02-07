//! Crawling domain actions
//!
//! All crawling operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they take raw Uuid types, handle conversions,
//! and return events directly.
//!
//! Use `ingest_website()` for crawling websites - it uses the extraction library's
//! Ingestor pattern with SSRF protection and integrated summarization.

pub mod authorization;
pub mod crawl_full;
pub mod ingest_website;
pub mod post_extraction;
pub mod regenerate_single_post;
pub mod website_context;

use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;
use extraction::types::page::CachedPage;

// Re-export helper functions
pub use authorization::check_crawl_authorization;
pub use crawl_full::crawl_website_full;
pub use ingest_website::{ingest_urls, ingest_website, IngestUrlsResult};
pub use post_extraction::{extract_narratives_for_domain, investigate_post};
pub use regenerate_single_post::regenerate_single_post;
pub use website_context::fetch_approved_website;

/// Regenerate posts from existing pages in extraction library.
/// Returns the fact event directly.
pub async fn regenerate_posts(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<CrawlEvent> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    // Auth check - returns error, not event
    check_crawl_authorization(requested_by, is_admin, "RegeneratePosts", deps).await?;

    // Fetch website (admin can regenerate regardless of approval status)
    let _website = Website::find_by_id(website_id_typed, &deps.db_pool)
        .await
        .map_err(|_| anyhow::anyhow!("Website not found"))?;

    // Count pages in extraction library to verify we have something to process
    let page_count = crate::domains::crawling::models::ExtractionPage::count_by_domain(
        &_website.domain,
        &deps.db_pool,
    )
    .await?;

    if page_count == 0 {
        return Err(anyhow::anyhow!(
            "No pages found in extraction library. Run a full crawl first."
        ));
    }

    info!(website_id = %website_id_typed, pages_count = page_count, "Triggering post regeneration");

    Ok(CrawlEvent::WebsitePostsRegenerated {
        website_id: website_id_typed,
        job_id,
        pages_processed: page_count,
    })
}

/// Discover pages using Tavily search instead of traditional crawling
/// Returns the fact event directly.
pub async fn discover_website(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<CrawlEvent> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(website_id = %website_id_typed, job_id = %job_id, "Starting Tavily-based discovery");

    // Auth check
    check_crawl_authorization(requested_by, is_admin, "DiscoverWebsite", deps).await?;

    // Fetch website
    let website = Website::find_by_id(website_id_typed, &deps.db_pool).await?;

    // Run Tavily discovery
    let max_pages = website.max_pages_per_crawl.unwrap_or(40) as usize;
    let discovered =
        match discover_pages(&website.domain, deps.web_searcher.as_ref(), max_pages).await {
            Ok(pages) => pages,
            Err(e) => {
                return Err(anyhow::anyhow!("Discovery failed: {}", e));
            }
        };

    if discovered.is_empty() {
        return Err(anyhow::anyhow!("No pages discovered via search"));
    }

    info!(
        website_id = %website_id_typed,
        discovered_count = discovered.len(),
        "Discovered pages via Tavily search"
    );

    // Get extraction service
    let extraction_service = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not configured"))?;

    // Convert discovered pages to CachedPage format for extraction library
    let site_url = format!("https://{}", website.domain);
    let cached_pages: Vec<CachedPage> = discovered
        .iter()
        .map(|page| {
            CachedPage::new(&page.url, &site_url, &page.content)
                .with_title(&page.title)
                .with_metadata("fetched_via", "tavily")
        })
        .collect();

    // Store in extraction_pages table
    let stored_count = extraction_service.store_pages(&cached_pages).await?;

    info!(
        website_id = %website_id_typed,
        stored_count = stored_count,
        "Stored pages in extraction_pages"
    );

    // Build event payload
    let crawled_pages: Vec<CrawledPageInfo> = discovered
        .into_iter()
        .map(|page| CrawledPageInfo {
            url: page.url,
            title: Some(page.title),
            snapshot_id: None, // extraction_pages uses URL as key, not UUID
        })
        .collect();

    info!(website_id = %website_id_typed, pages_stored = crawled_pages.len(), "Returning WebsitePagesDiscovered");

    Ok(CrawlEvent::WebsitePagesDiscovered {
        website_id: website_id_typed,
        job_id,
        pages: crawled_pages,
        discovery_method: "tavily".to_string(),
    })
}
