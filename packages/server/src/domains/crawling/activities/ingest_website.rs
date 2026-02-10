//! Website ingestion using the extraction library.
//!
//! This replaces the old crawl_website action with a simplified version
//! that uses the extraction library's Ingestor pattern.

use anyhow::Result;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::crawling::types::WebsiteIngested;
use crate::domains::website::models::Website;
use crate::kernel::{
    DiscoverConfig, FirecrawlIngestor, HttpIngestor, ServerDeps, ValidatedIngestor,
};

/// Result of an ingest_urls operation (no event emission)
#[derive(Debug, Clone)]
pub struct IngestUrlsResult {
    pub job_id: Uuid,
    pub website_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Ingest a website using the extraction library.
///
/// Uses Firecrawl if `FIRECRAWL_API_KEY` is set, otherwise falls back to HTTP.
pub async fn ingest_website(
    website_id: Uuid,
    visitor_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<WebsiteIngested> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(visitor_id);
    let job_id = JobId::new();
    let use_firecrawl = std::env::var("FIRECRAWL_API_KEY").ok().filter(|k| !k.is_empty()).is_some();

    info!(
        website_id = %website_id_typed,
        job_id = %job_id,
        use_firecrawl = %use_firecrawl,
        "Starting website ingestion via extraction library"
    );

    // 1. Auth check
    debug!(website_id = %website_id_typed, "Checking authorization");
    Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(deps)
        .await?;
    debug!(website_id = %website_id_typed, "Authorization passed");

    // 2. Fetch website
    debug!(website_id = %website_id_typed, "Fetching website from database");
    let website = Website::find_by_id(website_id_typed, &deps.db_pool).await?;
    debug!(website_id = %website_id_typed, domain = %website.domain, "Website fetched successfully");

    // 3. Get extraction service (required for ingestion)
    debug!(website_id = %website_id_typed, "Getting extraction service");
    let extraction = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // 5. Configure discovery
    let max_pages = 40usize;
    let max_depth = website.max_crawl_depth as usize;

    debug!(
        website_id = %website_id_typed,
        domain = %website.domain,
        max_pages = max_pages,
        max_depth = max_depth,
        "Configuring discovery"
    );

    // Ensure URL has scheme - domain is stored without scheme
    let url = if website.domain.starts_with("http://") || website.domain.starts_with("https://") {
        website.domain.clone()
    } else {
        format!("https://{}", website.domain)
    };

    let discover_config = DiscoverConfig::new(&url)
        .with_limit(max_pages)
        .with_max_depth(max_depth);

    // 6. Create ingestor and run ingestion
    info!(
        website_id = %website_id_typed,
        domain = %website.domain,
        use_firecrawl = use_firecrawl,
        "Starting ingestion with ingestor"
    );
    let ingest_result = if use_firecrawl {
        // Try Firecrawl first
        match FirecrawlIngestor::from_env() {
            Ok(firecrawl) => {
                info!(website_id = %website_id_typed, "Using Firecrawl ingestor");
                let ingestor = ValidatedIngestor::new(firecrawl);
                debug!(website_id = %website_id_typed, "Calling extraction.ingest() with Firecrawl");
                extraction.ingest(&discover_config, &ingestor).await
            }
            Err(e) => {
                warn!(error = %e, "Firecrawl not available, falling back to HTTP");
                let http = HttpIngestor::new();
                let ingestor = ValidatedIngestor::new(http);
                debug!(website_id = %website_id_typed, "Calling extraction.ingest() with HTTP (fallback)");
                extraction.ingest(&discover_config, &ingestor).await
            }
        }
    } else {
        info!(website_id = %website_id_typed, "Using HTTP ingestor");
        let http = HttpIngestor::new();
        let ingestor = ValidatedIngestor::new(http);
        debug!(website_id = %website_id_typed, "Calling extraction.ingest() with HTTP");
        extraction.ingest(&discover_config, &ingestor).await
    };

    let result = match ingest_result {
        Ok(r) => r,
        Err(e) => {
            return Err(anyhow::anyhow!("Ingestion failed: {}", e));
        }
    };

    info!(
        website_id = %website_id_typed,
        pages_crawled = result.pages_crawled,
        pages_summarized = result.pages_summarized,
        pages_skipped = result.pages_skipped,
        "Extraction library ingestion completed"
    );

    // Update website last_scraped_at timestamp
    Website::update_last_scraped(website_id_typed, &deps.db_pool).await?;

    Ok(WebsiteIngested {
        website_id: website_id_typed.into_uuid(),
        job_id: job_id.into_uuid(),
        pages_crawled: result.pages_crawled,
        pages_summarized: result.pages_summarized,
    })
}

/// Ingest specific URLs into the extraction library.
///
/// Used for:
/// - User-submitted URLs
/// - Gap-filling (fetching specific pages to answer questions)
/// - Adding individual pages to an existing website
///
/// Note: This function does not emit events - it's a utility for batch URL ingestion.
pub async fn ingest_urls(
    website_id: Uuid,
    urls: Vec<String>,
    visitor_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<IngestUrlsResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(visitor_id);
    let job_id = JobId::new();

    info!(
        website_id = %website_id_typed,
        job_id = %job_id,
        url_count = urls.len(),
        "Ingesting specific URLs via extraction library"
    );

    // Auth check
    Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(deps)
        .await?;

    // Get extraction service (required for ingestion)
    let extraction = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // Create ingestor - prefer Firecrawl for specific URLs
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
        Ok(r) => {
            info!(
                website_id = %website_id_typed,
                pages_summarized = r.pages_summarized,
                "URL ingestion completed"
            );
            Ok(IngestUrlsResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "completed".to_string(),
                message: Some(format!("Ingested {} URLs", r.pages_summarized)),
            })
        }
        Err(e) => Ok(IngestUrlsResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "failed".to_string(),
            message: Some(format!("URL ingestion failed: {}", e)),
        }),
    }
}
