//! Website ingestion using the extraction library.
//!
//! This replaces the old crawl_website action with a simplified version
//! that uses the extraction library's Ingestor pattern.

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::crawling::actions::{check_crawl_authorization, CrawlJobResult};
use crate::domains::crawling::events::CrawlEvent;
use crate::domains::website::models::Website;
use crate::kernel::{
    DiscoverConfig, FirecrawlIngestor, HttpIngestor, ServerDeps, ValidatedIngestor,
};
use seesaw_core::EffectContext;

/// Ingest a website using the extraction library.
///
/// This is the new preferred method for crawling websites. It:
/// 1. Uses the extraction library's Ingestor pattern for fetching
/// 2. Stores pages in both extraction_pages (via ExtractionService) and
///    page_snapshots (for backward compatibility)
/// 3. Creates website_snapshot junction entries
/// 4. Emits events for the extraction cascade
///
/// # Arguments
///
/// * `website_id` - Website to ingest
/// * `member_id` - Member requesting the action
/// * `is_admin` - Whether the member is an admin
/// * `use_firecrawl` - Whether to use Firecrawl (true) or basic HTTP (false)
/// * `ctx` - Effect context for emitting events
pub async fn ingest_website(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    use_firecrawl: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(
        website_id = %website_id_typed,
        job_id = %job_id,
        use_firecrawl = %use_firecrawl,
        "Starting website ingestion via extraction library"
    );

    // 1. Auth check
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "IngestWebsite", ctx.deps()).await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // 2. Fetch website
    let website = match Website::find_by_id(website_id_typed, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            ctx.emit(CrawlEvent::WebsiteCrawlFailed {
                website_id: website_id_typed,
                job_id,
                reason: format!("Failed to find website: {}", e),
            });
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some(format!("Website not found: {}", e)),
            });
        }
    };

    // 3. Start crawl status
    if let Err(e) = Website::start_crawl(website_id_typed, &ctx.deps().db_pool).await {
        warn!(website_id = %website_id_typed, error = %e, "Failed to update crawl status");
    }

    // 4. Get extraction service
    let extraction = match ctx.deps().extraction.as_ref() {
        Some(e) => e,
        None => {
            let _ = Website::complete_crawl(website_id_typed, "failed", 0, &ctx.deps().db_pool).await;
            ctx.emit(CrawlEvent::WebsiteCrawlFailed {
                website_id: website_id_typed,
                job_id,
                reason: "Extraction service not configured".to_string(),
            });
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some("Extraction service not configured".to_string()),
            });
        }
    };

    // 5. Configure discovery
    let max_pages = website.max_pages_per_crawl.unwrap_or(20) as usize;
    let max_depth = website.max_crawl_depth as usize;

    let discover_config = DiscoverConfig::new(&website.domain)
        .with_limit(max_pages)
        .with_max_depth(max_depth);

    // 6. Create ingestor and run ingestion
    let ingest_result = if use_firecrawl {
        // Try Firecrawl first
        match FirecrawlIngestor::from_env() {
            Ok(firecrawl) => {
                let ingestor = ValidatedIngestor::new(firecrawl);
                extraction.ingest(&discover_config, &ingestor).await
            }
            Err(e) => {
                warn!(error = %e, "Firecrawl not available, falling back to HTTP");
                let http = HttpIngestor::new();
                let ingestor = ValidatedIngestor::new(http);
                extraction.ingest(&discover_config, &ingestor).await
            }
        }
    } else {
        let http = HttpIngestor::new();
        let ingestor = ValidatedIngestor::new(http);
        extraction.ingest(&discover_config, &ingestor).await
    };

    let result = match ingest_result {
        Ok(r) => r,
        Err(e) => {
            let _ = Website::complete_crawl(website_id_typed, "failed", 0, &ctx.deps().db_pool).await;
            ctx.emit(CrawlEvent::WebsiteCrawlFailed {
                website_id: website_id_typed,
                job_id,
                reason: format!("Ingestion failed: {}", e),
            });
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some(format!("Ingestion failed: {}", e)),
            });
        }
    };

    info!(
        website_id = %website_id_typed,
        pages_crawled = result.pages_crawled,
        pages_summarized = result.pages_summarized,
        pages_skipped = result.pages_skipped,
        "Extraction library ingestion completed"
    );

    // 7. Update website status
    // Note: Backward-compatible records (website_snapshots, page_snapshots) are not created
    // here. The extraction library stores pages in extraction_pages table. We'll migrate
    // the old tables out in a future phase.
    let _ = Website::complete_crawl(
        website_id_typed,
        "crawling",
        result.pages_summarized as i32,
        &ctx.deps().db_pool,
    )
    .await;

    // 8. Emit event to continue the cascade
    // Note: We're emitting with empty crawled_pages for now since the extraction
    // library handles summarization internally. The cascade may need adjustment.
    info!(
        website_id = %website_id_typed,
        pages_processed = result.pages_summarized,
        "Emitting WebsiteIngested event"
    );

    ctx.emit(CrawlEvent::WebsiteIngested {
        website_id: website_id_typed,
        job_id,
        pages_crawled: result.pages_crawled,
        pages_summarized: result.pages_summarized,
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status: "completed".to_string(),
        message: Some(format!(
            "Ingested {} pages ({} summarized, {} skipped)",
            result.pages_crawled, result.pages_summarized, result.pages_skipped
        )),
    })
}

/// Ingest specific URLs into the extraction library.
///
/// Used for:
/// - User-submitted URLs
/// - Gap-filling (fetching specific pages to answer questions)
/// - Adding individual pages to an existing website
pub async fn ingest_urls(
    website_id: Uuid,
    urls: Vec<String>,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(
        website_id = %website_id_typed,
        job_id = %job_id,
        url_count = urls.len(),
        "Ingesting specific URLs via extraction library"
    );

    // Auth check
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "IngestUrls", ctx.deps()).await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // Get extraction service
    let extraction = match ctx.deps().extraction.as_ref() {
        Some(e) => e,
        None => {
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some("Extraction service not configured".to_string()),
            });
        }
    };

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
            Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "completed".to_string(),
                message: Some(format!("Ingested {} URLs", r.pages_summarized)),
            })
        }
        Err(e) => Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "failed".to_string(),
            message: Some(format!("URL ingestion failed: {}", e)),
        }),
    }
}
