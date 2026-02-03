//! Crawling domain actions
//!
//! All crawling operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they take raw Uuid types, handle conversions,
//! and return results directly.
//!
//! Use `ingest_website()` for crawling websites - it uses the extraction library's
//! Ingestor pattern with SSRF protection and integrated summarization.

pub mod authorization;
pub mod build_pages;
pub mod ingest_website;
pub mod regenerate_page;
pub mod sync_posts;
pub mod website_context;

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::crawling::effects::discovery::discover_pages;
use crate::domains::crawling::effects::extraction::summarize_pages;
use crate::domains::crawling::events::{CrawlEvent, CrawledPageInfo};
use crate::domains::crawling::models::{PageSnapshot, PageSummary};
use crate::domains::website::models::{Website, WebsiteSnapshot};
use crate::kernel::ServerDeps;

// Re-export helper functions
pub use authorization::check_crawl_authorization;
pub use build_pages::{
    build_page_to_summarize_from_snapshot, build_pages_to_summarize, fetch_single_page_context,
    SinglePageContext,
};
pub use ingest_website::{ingest_urls, ingest_website};
pub use regenerate_page::{regenerate_posts_for_page, regenerate_summary_for_page};
pub use sync_posts::{
    llm_deduplicate_website_posts, sync_and_deduplicate_posts, SyncAndDedupResult,
};
pub use website_context::{fetch_approved_website, fetch_snapshots_as_crawled_pages};

/// Result of a crawl/regenerate operation
#[derive(Debug, Clone)]
pub struct CrawlJobResult {
    pub job_id: Uuid,
    pub website_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Crawl a website (multi-page)
/// Returns job result directly.
///
/// # Deprecated
/// Use `ingest_website()` instead which uses the extraction library's
/// Ingestor pattern. The new function provides:
/// - Pluggable ingestors (HTTP, Firecrawl)
/// - SSRF protection via ValidatedIngestor
/// - Integrated summarization and embedding
/// - Storage in extraction_pages table (not page_snapshots)
///
/// This function now delegates to `ingest_website()`.
#[deprecated(since = "0.1.0", note = "Use ingest_website() instead")]
pub async fn crawl_website(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    // Delegate to ingest_website with Firecrawl disabled (basic HTTP crawling)
    ingest_website(website_id, member_id, is_admin, false, ctx).await
}

/// Regenerate posts from existing page snapshots
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

    // Auth check
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "RegeneratePosts", ctx.deps()).await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // Fetch approved website
    if fetch_approved_website(website_id_typed, &ctx.deps().db_pool)
        .await
        .is_none()
    {
        ctx.emit(CrawlEvent::WebsiteCrawlFailed {
            website_id: website_id_typed,
            job_id,
            reason: "Website not found or not approved".to_string(),
        });
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "failed".to_string(),
            message: Some("Website not found or not approved".to_string()),
        });
    }

    // Get existing snapshots as crawled pages
    let crawled_pages =
        fetch_snapshots_as_crawled_pages(website_id_typed, &ctx.deps().db_pool).await;
    if crawled_pages.is_empty() {
        ctx.emit(CrawlEvent::WebsiteCrawlFailed {
            website_id: website_id_typed,
            job_id,
            reason: "No page snapshots found. Run a full crawl first.".to_string(),
        });
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "failed".to_string(),
            message: Some("No page snapshots found".to_string()),
        });
    }

    info!(website_id = %website_id_typed, pages_count = crawled_pages.len(), "Triggering extraction");
    ctx.emit(CrawlEvent::WebsiteCrawled {
        website_id: website_id_typed,
        job_id,
        pages: crawled_pages,
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status: "completed".to_string(),
        message: Some("Regeneration completed".to_string()),
    })
}

/// Regenerate AI summaries for all pages of a website
/// Returns job result directly.
pub async fn regenerate_page_summaries(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    // Auth check
    if let Err(event) = check_crawl_authorization(
        requested_by,
        is_admin,
        "RegeneratePageSummaries",
        ctx.deps(),
    )
    .await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // Fetch approved website
    if fetch_approved_website(website_id_typed, &ctx.deps().db_pool)
        .await
        .is_none()
    {
        ctx.emit(CrawlEvent::WebsiteCrawlFailed {
            website_id: website_id_typed,
            job_id,
            reason: "Website not found or not approved".to_string(),
        });
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "failed".to_string(),
            message: Some("Website not found or not approved".to_string()),
        });
    }

    // Get snapshots and delete cached summaries
    let crawled_pages =
        fetch_snapshots_as_crawled_pages(website_id_typed, &ctx.deps().db_pool).await;
    for page in &crawled_pages {
        if let Some(ps_id) = page.snapshot_id {
            let _ = PageSummary::delete_for_snapshot(ps_id, &ctx.deps().db_pool).await;
        }
    }

    // Build pages to summarize
    let (pages_to_summarize, _) =
        match build_pages_to_summarize(&crawled_pages, &ctx.deps().db_pool).await {
            Ok(result) => result,
            Err(e) => {
                ctx.emit(CrawlEvent::WebsiteCrawlFailed {
                    website_id: website_id_typed,
                    job_id,
                    reason: format!("Failed to build pages: {}", e),
                });
                return Ok(CrawlJobResult {
                    job_id: job_id.into_uuid(),
                    website_id,
                    status: "failed".to_string(),
                    message: Some(format!("Failed to build pages: {}", e)),
                });
            }
        };

    if pages_to_summarize.is_empty() {
        ctx.emit(CrawlEvent::WebsiteCrawlFailed {
            website_id: website_id_typed,
            job_id,
            reason: "No page snapshots with content found.".to_string(),
        });
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "failed".to_string(),
            message: Some("No page snapshots with content found".to_string()),
        });
    }

    // Run summarization
    let summaries = match summarize_pages(
        pages_to_summarize,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            ctx.emit(CrawlEvent::WebsiteCrawlFailed {
                website_id: website_id_typed,
                job_id,
                reason: format!("Summarization failed: {}", e),
            });
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some(format!("Summarization failed: {}", e)),
            });
        }
    };

    info!(website_id = %website_id_typed, summaries = summaries.len(), "Page summaries regenerated");
    ctx.emit(CrawlEvent::PageSummariesRegenerated {
        website_id: website_id_typed,
        job_id,
        pages_processed: summaries.len(),
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status: "completed".to_string(),
        message: Some(format!("Page summaries regenerated ({})", summaries.len())),
    })
}

/// Regenerate AI summary for a single page
/// Returns job result directly.
pub async fn regenerate_page_summary(
    page_snapshot_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    // Auth check
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "RegeneratePageSummary", ctx.deps()).await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id: page_snapshot_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // Delegate to helper
    regenerate_summary_for_page(page_snapshot_id, ctx.deps()).await;
    ctx.emit(CrawlEvent::PageSummaryRegenerated {
        page_snapshot_id,
        job_id,
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id: page_snapshot_id,
        status: "completed".to_string(),
        message: Some("AI summary regenerated".to_string()),
    })
}

/// Regenerate posts for a single page
/// Returns job result directly.
pub async fn regenerate_page_posts(
    page_snapshot_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    // Auth check
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "RegeneratePagePosts", ctx.deps()).await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id: page_snapshot_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // Delegate to helper
    let posts_count = regenerate_posts_for_page(page_snapshot_id, job_id, ctx.deps()).await;
    ctx.emit(CrawlEvent::PagePostsRegenerated {
        page_snapshot_id,
        job_id,
        posts_count,
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id: page_snapshot_id,
        status: "completed".to_string(),
        message: Some(format!("Posts regenerated ({})", posts_count)),
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
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "DiscoverWebsite", ctx.deps()).await
    {
        ctx.emit(event);
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "auth_failed".to_string(),
            message: Some("Authorization denied".to_string()),
        });
    }

    // Fetch website
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

    // Start crawl status tracking
    if let Err(e) = Website::start_crawl(website_id_typed, &ctx.deps().db_pool).await {
        warn!(website_id = %website_id_typed, error = %e, "Failed to update crawl status");
    }

    // Run Tavily discovery
    let max_pages = website.max_pages_per_crawl.unwrap_or(20) as usize;
    let discovered = match discover_pages(
        &website.domain,
        ctx.deps().web_searcher.as_ref(),
        max_pages,
    )
    .await
    {
        Ok(pages) => pages,
        Err(e) => {
            let _ =
                Website::complete_crawl(website_id_typed, "failed", 0, &ctx.deps().db_pool).await;
            ctx.emit(CrawlEvent::WebsiteCrawlFailed {
                website_id: website_id_typed,
                job_id,
                reason: format!("Discovery failed: {}", e),
            });
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some(format!("Discovery failed: {}", e)),
            });
        }
    };

    if discovered.is_empty() {
        let _ = Website::complete_crawl(
            website_id_typed,
            "no_listings_found",
            0,
            &ctx.deps().db_pool,
        )
        .await;
        ctx.emit(CrawlEvent::WebsiteCrawlFailed {
            website_id: website_id_typed,
            job_id,
            reason: "No pages discovered via search".to_string(),
        });
        return Ok(CrawlJobResult {
            job_id: job_id.into_uuid(),
            website_id,
            status: "no_pages".to_string(),
            message: Some("No pages discovered via search".to_string()),
        });
    }

    info!(
        website_id = %website_id_typed,
        discovered_count = discovered.len(),
        "Discovered pages via Tavily search"
    );

    // Store discovered pages as snapshots
    let mut crawled_pages = Vec::new();
    let pool = &ctx.deps().db_pool;

    for page in discovered {
        // Create page snapshot with Tavily content
        // Note: We pass content as both html and markdown since Tavily gives us clean text
        let (page_snapshot, _is_new) = match PageSnapshot::upsert(
            pool,
            page.url.clone(),
            page.content.clone(),       // html (using content as placeholder)
            Some(page.content.clone()), // markdown (the actual extracted content)
            "tavily".to_string(),       // fetched_via
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                warn!(url = %page.url, error = %e, "Failed to create page snapshot");
                continue;
            }
        };

        // Create website_snapshot entry and link to page snapshot
        match WebsiteSnapshot::upsert(pool, website_id_typed, page.url.clone(), None).await {
            Ok(website_snapshot) => {
                if let Err(e) = website_snapshot.link_snapshot(pool, page_snapshot.id).await {
                    warn!(url = %page.url, error = %e, "Failed to link website_snapshot to page_snapshot");
                }
            }
            Err(e) => {
                warn!(url = %page.url, error = %e, "Failed to create website snapshot");
            }
        }

        crawled_pages.push(CrawledPageInfo {
            url: page.url,
            title: Some(page.title),
            snapshot_id: Some(page_snapshot.id),
        });
    }

    // Update website status
    let _ = Website::complete_crawl(
        website_id_typed,
        "crawling",
        crawled_pages.len() as i32,
        &ctx.deps().db_pool,
    )
    .await;

    let pages_count = crawled_pages.len();
    info!(website_id = %website_id_typed, pages_stored = pages_count, "Emitting WebsiteCrawled from discovery");

    // Emit WebsiteCrawled event to trigger extraction cascade
    ctx.emit(CrawlEvent::WebsiteCrawled {
        website_id: website_id_typed,
        job_id,
        pages: crawled_pages,
    });

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status: "completed".to_string(),
        message: Some(format!("Discovered {} pages via search", pages_count)),
    })
}
