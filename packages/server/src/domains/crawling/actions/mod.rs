//! Crawling domain actions
//!
//! All crawling operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they take raw Uuid types, handle conversions,
//! and return results directly.
//!
//! Use `ingest_website()` for crawling websites - it uses the extraction library's
//! Ingestor pattern with SSRF protection and integrated summarization.

pub mod authorization;
pub mod ingest_website;
pub mod post_extraction;
pub mod sync_posts;
pub mod website_context;

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::crawling::effects::discovery::discover_pages;
use crate::domains::crawling::events::{CrawlEvent, CrawledPageInfo};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;
use extraction::types::page::CachedPage;

// Re-export helper functions
pub use authorization::check_crawl_authorization;
pub use ingest_website::{ingest_urls, ingest_website};
pub use sync_posts::{sync_and_deduplicate_posts, SyncAndDedupResult};
pub use website_context::fetch_approved_website;

/// Result of a crawl/regenerate operation
#[derive(Debug, Clone)]
pub struct CrawlJobResult {
    pub job_id: Uuid,
    pub website_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Regenerate posts from existing pages in extraction library.
/// Returns job result directly.
pub async fn regenerate_posts(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    // Auth check - returns error, not event
    check_crawl_authorization(requested_by, is_admin, "RegeneratePosts", ctx.deps()).await?;

    // Fetch approved website
    let _website = fetch_approved_website(website_id_typed, &ctx.deps().db_pool)
        .await
        .ok_or_else(|| anyhow::anyhow!("Website not found or not approved"))?;

    // Count pages in extraction library to verify we have something to process
    let page_count = crate::domains::crawling::models::ExtractionPage::count_by_domain(
        &_website.domain,
        &ctx.deps().db_pool,
    )
    .await?;

    if page_count == 0 {
        return Err(anyhow::anyhow!(
            "No pages found in extraction library. Run a full crawl first."
        ));
    }

    info!(website_id = %website_id_typed, pages_count = page_count, "Triggering post regeneration");

    // Emit fact event - posts are being regenerated from existing pages
    ctx.emit(CrawlEvent::WebsitePostsRegenerated {
        website_id: website_id_typed,
        job_id,
        pages_processed: page_count,
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status: "completed".to_string(),
        message: Some(format!("Regeneration triggered for {} pages", page_count)),
    })
}

/// Discover pages using Tavily search instead of traditional crawling
/// Returns job result directly.
pub async fn discover_website(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(website_id = %website_id_typed, job_id = %job_id, "Starting Tavily-based discovery");

    // Auth check
    check_crawl_authorization(requested_by, is_admin, "DiscoverWebsite", ctx.deps()).await?;

    // Fetch website
    let website = Website::find_by_id(website_id_typed, &ctx.deps().db_pool).await?;

    // Run Tavily discovery
    let max_pages = website.max_pages_per_crawl.unwrap_or(20) as usize;
    let discovered =
        match discover_pages(&website.domain, ctx.deps().web_searcher.as_ref(), max_pages).await {
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
    let extraction_service = ctx
        .deps()
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

    let pages_count = crawled_pages.len();
    info!(website_id = %website_id_typed, pages_stored = pages_count, "Emitting WebsitePagesDiscovered");

    // Emit fact event - pages were discovered via search
    ctx.emit(CrawlEvent::WebsitePagesDiscovered {
        website_id: website_id_typed,
        job_id,
        pages: crawled_pages,
        discovery_method: "tavily".to_string(),
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status: "completed".to_string(),
        message: Some(format!("Discovered {} pages via search", pages_count)),
    })
}
