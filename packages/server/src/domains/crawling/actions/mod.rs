//! Crawling domain actions
//!
//! All crawling operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they take raw Uuid types, handle conversions,
//! and return results directly.

pub mod authorization;
pub mod build_pages;
pub mod crawl_website;
pub mod extract_posts;
pub mod regenerate_page;
pub mod sync_posts;
pub mod website_context;

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::crawling::effects::extraction::summarize_pages;
use crate::domains::crawling::events::CrawlEvent;
use crate::domains::crawling::models::PageSummary;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

// Re-export helper functions
pub use authorization::check_crawl_authorization;
pub use build_pages::{
    build_page_to_summarize_from_snapshot, build_pages_to_summarize, fetch_single_page_context,
    SinglePageContext,
};
pub use crawl_website::{crawl_website_pages, get_crawl_priorities, store_crawled_pages};
pub use extract_posts::{extract_posts_from_pages, update_page_extraction_status, ExtractionResult};
pub use regenerate_page::{regenerate_posts_for_page, regenerate_summary_for_page};
pub use sync_posts::{llm_deduplicate_website_posts, sync_and_deduplicate_posts, SyncAndDedupResult};
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
pub async fn crawl_website(
    website_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<CrawlJobResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(website_id = %website_id_typed, job_id = %job_id, "Starting multi-page crawl");

    // Auth check
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "CrawlWebsite", ctx.deps()).await
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

    // Start crawl
    if let Err(e) = Website::start_crawl(website_id_typed, &ctx.deps().db_pool).await {
        warn!(website_id = %website_id_typed, error = %e, "Failed to update crawl status");
    }

    // Crawl and store pages
    let crawled_pages = match crawl_website_pages(
        &website,
        job_id,
        ctx.deps().web_scraper.as_ref(),
        ctx.deps(),
    )
    .await
    {
        Ok(pages) => pages,
        Err(event) => {
            ctx.emit(event);
            return Ok(CrawlJobResult {
                job_id: job_id.into_uuid(),
                website_id,
                status: "failed".to_string(),
                message: Some("Crawl failed".to_string()),
            });
        }
    };

    // Update website status
    let _ = Website::complete_crawl(
        website_id_typed,
        "crawling",
        crawled_pages.len() as i32,
        &ctx.deps().db_pool,
    )
    .await;

    info!(website_id = %website_id_typed, pages_stored = crawled_pages.len(), "Emitting WebsiteCrawled");

    // Emit WebsiteCrawled event to trigger extraction cascade
    ctx.emit(CrawlEvent::WebsiteCrawled {
        website_id: website_id_typed,
        job_id,
        pages: crawled_pages,
    });

    // Check final status
    let final_website = Website::find_by_id(website_id_typed, &ctx.deps().db_pool).await.ok();
    let (status, message) = match final_website.map(|w| w.status) {
        Some(ref s) if s == "no_posts_found" => (
            "no_posts".to_string(),
            Some("No listings found".to_string()),
        ),
        _ => (
            "completed".to_string(),
            Some("Crawl completed successfully".to_string()),
        ),
    };

    Ok(CrawlJobResult {
        job_id: job_id.into_uuid(),
        website_id,
        status,
        message,
    })
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
    let crawled_pages = fetch_snapshots_as_crawled_pages(website_id_typed, &ctx.deps().db_pool).await;
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
    if let Err(event) =
        check_crawl_authorization(requested_by, is_admin, "RegeneratePageSummaries", ctx.deps())
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
    let crawled_pages = fetch_snapshots_as_crawled_pages(website_id_typed, &ctx.deps().db_pool).await;
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
    let summaries =
        match summarize_pages(pages_to_summarize, ctx.deps().ai.as_ref(), &ctx.deps().db_pool).await
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
