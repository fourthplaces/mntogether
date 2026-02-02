//! CrawlerEffect - Handles multi-page website crawling workflow
//!
//! This effect handles CrawlEvent request events and dispatches to handler functions.
//! Following CLAUDE.md: Effects must be thin orchestration layers, business logic in actions.
//!
//! Request events handled:
//! - CrawlWebsiteRequested: Crawl multiple pages from a website
//! - ExtractPostsFromPagesRequested: Extract posts from all crawled pages
//! - RetryWebsiteCrawlRequested: Retry crawl after no posts found
//! - MarkWebsiteNoPostsRequested: Mark website as having no posts (terminal)
//! - SyncCrawledPostsRequested: Sync extracted posts to database
//! - RegeneratePostsRequested: Regenerate posts from existing snapshots
//! - RegeneratePageSummariesRequested: Regenerate AI summaries for pages
//! - RegeneratePageSummaryRequested: Regenerate AI summary for single page
//! - RegeneratePagePostsRequested: Regenerate posts for single page

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::{info, warn};

use crate::common::{ExtractedPost, JobId, MemberId, WebsiteId};
use crate::domains::crawling::actions;
use crate::domains::crawling::events::{CrawledPageInfo, CrawlEvent, PageExtractionResult};
use crate::domains::crawling::models::PageSummary;
use crate::domains::crawling::effects::extraction::summarize_pages;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Crawler Effect - Handles CrawlEvent request events
///
/// This effect is a thin orchestration layer that dispatches request events to handler functions.
/// Fact events should never reach this effect (they're outputs, not inputs).
pub struct CrawlerEffect;

#[async_trait]
impl Effect<CrawlEvent, ServerDeps> for CrawlerEffect {
    type Event = CrawlEvent;

    async fn handle(
        &mut self,
        event: CrawlEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<Option<CrawlEvent>> {
        match event {
            // =================================================================
            // Request Events → Dispatch to Handlers
            // =================================================================
            CrawlEvent::CrawlWebsiteRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_crawl_website(website_id, job_id, requested_by, is_admin, &ctx).await.map(Some),

            CrawlEvent::ExtractPostsFromPagesRequested {
                website_id,
                job_id,
                pages,
            } => handle_extract_from_pages(website_id, job_id, pages, &ctx).await.map(Some),

            CrawlEvent::RetryWebsiteCrawlRequested { website_id, job_id } => {
                handle_retry_crawl(website_id, job_id, &ctx).await.map(Some)
            }

            CrawlEvent::MarkWebsiteNoPostsRequested { website_id, job_id } => {
                handle_mark_no_posts(website_id, job_id, &ctx).await.map(Some)
            }

            CrawlEvent::SyncCrawledPostsRequested {
                website_id,
                job_id,
                posts,
                page_results,
            } => handle_sync_crawled_posts(website_id, job_id, posts, page_results, &ctx).await.map(Some),

            CrawlEvent::RegeneratePostsRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_regenerate_posts(website_id, job_id, requested_by, is_admin, &ctx).await.map(Some),

            CrawlEvent::RegeneratePageSummariesRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                handle_regenerate_page_summaries(website_id, job_id, requested_by, is_admin, &ctx)
                    .await
                    .map(Some)
            }

            CrawlEvent::RegeneratePageSummaryRequested {
                page_snapshot_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                handle_regenerate_single_page_summary(
                    page_snapshot_id,
                    job_id,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            CrawlEvent::RegeneratePagePostsRequested {
                page_snapshot_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                handle_regenerate_single_page_posts(
                    page_snapshot_id,
                    job_id,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
                .map(Some)
            }

            // =================================================================
            // Fact Events → Terminal, no follow-up needed
            // =================================================================
            CrawlEvent::WebsiteCrawled { .. }
            | CrawlEvent::PagesReadyForExtraction { .. }
            | CrawlEvent::WebsiteCrawlNoListings { .. }
            | CrawlEvent::WebsiteMarkedNoListings { .. }
            | CrawlEvent::WebsiteCrawlFailed { .. }
            | CrawlEvent::PostsExtractedFromPages { .. }
            | CrawlEvent::PostsSynced { .. }
            | CrawlEvent::PageSummariesRegenerated { .. }
            | CrawlEvent::PageSummaryRegenerated { .. }
            | CrawlEvent::PagePostsRegenerated { .. }
            | CrawlEvent::AuthorizationDenied { .. } => Ok(None),
        }
    }
}

// ============================================================================
// Handler: CrawlWebsite (~30 lines)
// ============================================================================

async fn handle_crawl_website(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(website_id = %website_id, job_id = %job_id, "Starting multi-page crawl");

    // Auth check
    if let Err(event) = actions::check_crawl_authorization(
        requested_by, is_admin, "CrawlWebsite", ctx.deps()
    ).await {
        return Ok(event);
    }

    // Fetch website
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id, job_id, reason: format!("Failed to find website: {}", e),
        }),
    };

    // Start crawl
    if let Err(e) = Website::start_crawl(website_id, &ctx.deps().db_pool).await {
        warn!(website_id = %website_id, error = %e, "Failed to update crawl status");
    }

    // Crawl and store pages
    let crawled_pages = match actions::crawl_website_pages(
        &website, job_id, ctx.deps().web_scraper.as_ref(), ctx.deps()
    ).await {
        Ok(pages) => pages,
        Err(event) => return Ok(event),
    };

    // Update website status
    let _ = Website::complete_crawl(
        website_id, "crawling", crawled_pages.len() as i32, &ctx.deps().db_pool
    ).await;

    // Collect page snapshot IDs
    let page_snapshot_ids: Vec<uuid::Uuid> = crawled_pages
        .iter()
        .filter_map(|p| p.snapshot_id)
        .collect();

    info!(website_id = %website_id, pages_stored = crawled_pages.len(), "Emitting PagesReadyForExtraction");

    Ok(CrawlEvent::PagesReadyForExtraction { website_id, job_id, page_snapshot_ids })
}

// ============================================================================
// Handler: ExtractPostsFromPages (~40 lines)
// ============================================================================

async fn handle_extract_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    pages: Vec<CrawledPageInfo>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(website_id = %website_id, pages_count = pages.len(), "Extracting posts");

    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;

    // Build pages to summarize
    let (pages_to_summarize, _) = actions::build_pages_to_summarize(&pages, &ctx.deps().db_pool).await?;

    if pages_to_summarize.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlNoListings {
            website_id, job_id, attempt_number: 1, pages_crawled: 0, should_retry: false,
        });
    }

    // Extract posts using two-pass extraction
    let result = match actions::extract_posts_from_pages(
        &website, pages_to_summarize, job_id, ctx.deps().ai.as_ref(), ctx.deps()
    ).await {
        Ok(r) => r,
        Err(event) => return Ok(event),
    };

    // Update page extraction status
    actions::update_page_extraction_status(&result.page_results, &ctx.deps().db_pool).await;

    // Check if we found any posts
    if result.posts.is_empty() {
        let attempt_count = Website::increment_crawl_attempt(website_id, &ctx.deps().db_pool)
            .await.unwrap_or(1);
        let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
        return Ok(CrawlEvent::WebsiteCrawlNoListings {
            website_id, job_id,
            attempt_number: attempt_count, pages_crawled: pages.len(),
            should_retry: website.should_retry_crawl(),
        });
    }

    info!(website_id = %website_id, total_posts = result.posts.len(), "Extraction complete");
    let _ = Website::reset_crawl_attempts(website_id, &ctx.deps().db_pool).await;

    Ok(CrawlEvent::PostsExtractedFromPages {
        website_id, job_id, posts: result.posts, page_results: result.page_results,
    })
}

// ============================================================================
// Handler: RetryWebsiteCrawl (~15 lines)
// ============================================================================

async fn handle_retry_crawl(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(website_id = %website_id, "Retrying website crawl");
    let _ = Website::reset_for_retry(website_id, &ctx.deps().db_pool).await;

    Ok(CrawlEvent::CrawlWebsiteRequested {
        website_id, job_id, requested_by: MemberId::new(), is_admin: true,
    })
}

// ============================================================================
// Handler: MarkWebsiteNoPosts (~20 lines)
// ============================================================================

async fn handle_mark_no_posts(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(website_id = %website_id, "Marking website as having no posts");

    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
    let total_attempts = website.crawl_attempt_count.unwrap_or(0);

    let _ = Website::complete_crawl(
        website_id, "no_posts_found", website.pages_crawled_count.unwrap_or(0), &ctx.deps().db_pool
    ).await;

    Ok(CrawlEvent::WebsiteMarkedNoListings { website_id, job_id, total_attempts })
}

// ============================================================================
// Handler: SyncCrawledPosts (~25 lines)
// ============================================================================

async fn handle_sync_crawled_posts(
    website_id: WebsiteId,
    job_id: JobId,
    posts: Vec<ExtractedPost>,
    page_results: Vec<PageExtractionResult>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(website_id = %website_id, posts_count = posts.len(), "Syncing crawled posts");

    // Sync and deduplicate
    let result = actions::sync_and_deduplicate_posts(website_id, posts, ctx.deps()).await?;

    // Mark crawl as completed
    let _ = Website::complete_crawl(
        website_id, "completed", page_results.len() as i32, &ctx.deps().db_pool
    ).await;

    Ok(CrawlEvent::PostsSynced {
        website_id, job_id,
        new_count: result.sync_result.new_count,
        updated_count: result.sync_result.updated_count,
        unchanged_count: result.sync_result.unchanged_count,
    })
}

// ============================================================================
// Handler: RegeneratePosts (~35 lines)
// ============================================================================

async fn handle_regenerate_posts(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    // Auth check
    if let Err(event) = actions::check_crawl_authorization(
        requested_by, is_admin, "RegeneratePosts", ctx.deps()
    ).await {
        return Ok(event);
    }

    // Fetch approved website
    if actions::fetch_approved_website(website_id, &ctx.deps().db_pool).await.is_none() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id, job_id, reason: "Website not found or not approved".to_string(),
        });
    }

    // Get existing snapshots as crawled pages
    let crawled_pages = actions::fetch_snapshots_as_crawled_pages(website_id, &ctx.deps().db_pool).await;
    if crawled_pages.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id, job_id, reason: "No page snapshots found. Run a full crawl first.".to_string(),
        });
    }

    info!(website_id = %website_id, pages_count = crawled_pages.len(), "Triggering extraction");
    Ok(CrawlEvent::WebsiteCrawled { website_id, job_id, pages: crawled_pages })
}

// ============================================================================
// Handler: RegeneratePageSummaries (~45 lines)
// ============================================================================

async fn handle_regenerate_page_summaries(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    // Auth check
    if let Err(event) = actions::check_crawl_authorization(
        requested_by, is_admin, "RegeneratePageSummaries", ctx.deps()
    ).await {
        return Ok(event);
    }

    // Fetch approved website
    if actions::fetch_approved_website(website_id, &ctx.deps().db_pool).await.is_none() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id, job_id, reason: "Website not found or not approved".to_string(),
        });
    }

    // Get snapshots and delete cached summaries
    let crawled_pages = actions::fetch_snapshots_as_crawled_pages(website_id, &ctx.deps().db_pool).await;
    for page in &crawled_pages {
        if let Some(ps_id) = page.snapshot_id {
            let _ = PageSummary::delete_for_snapshot(ps_id, &ctx.deps().db_pool).await;
        }
    }

    // Build pages to summarize
    let (pages_to_summarize, _) = actions::build_pages_to_summarize(&crawled_pages, &ctx.deps().db_pool).await?;
    if pages_to_summarize.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id, job_id, reason: "No page snapshots with content found.".to_string(),
        });
    }

    // Run summarization
    let summaries = summarize_pages(pages_to_summarize, ctx.deps().ai.as_ref(), &ctx.deps().db_pool).await
        .map_err(|e| anyhow::anyhow!("Summarization failed: {}", e))?;

    info!(website_id = %website_id, summaries = summaries.len(), "Page summaries regenerated");
    Ok(CrawlEvent::PageSummariesRegenerated { website_id, job_id, pages_processed: summaries.len() })
}

// ============================================================================
// Handler: RegeneratePageSummary (single page) (~35 lines)
// ============================================================================

async fn handle_regenerate_single_page_summary(
    page_snapshot_id: uuid::Uuid,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    // Auth check
    if let Err(event) = actions::check_crawl_authorization(
        requested_by, is_admin, "RegeneratePageSummary", ctx.deps()
    ).await {
        return Ok(event);
    }

    // Delegate to action
    actions::regenerate_summary_for_page(page_snapshot_id, ctx.deps()).await;
    Ok(CrawlEvent::PageSummaryRegenerated { page_snapshot_id, job_id })
}

// ============================================================================
// Handler: RegeneratePagePosts (single page) (~50 lines)
// ============================================================================

async fn handle_regenerate_single_page_posts(
    page_snapshot_id: uuid::Uuid,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    // Auth check
    if let Err(event) = actions::check_crawl_authorization(
        requested_by, is_admin, "RegeneratePagePosts", ctx.deps()
    ).await {
        return Ok(event);
    }

    // Delegate to action
    let posts_count = actions::regenerate_posts_for_page(page_snapshot_id, job_id, ctx.deps()).await;
    Ok(CrawlEvent::PagePostsRegenerated { page_snapshot_id, job_id, posts_count })
}
