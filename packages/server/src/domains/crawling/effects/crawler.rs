//! CrawlerEffect - Handles multi-page website crawling workflow
//!
//! This effect handles CrawlEvent request events and dispatches to handler functions.
//! Following CLAUDE.md: Effects must be thin orchestration layers, business logic in handlers.
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

use crate::kernel::ServerDeps;
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{ContactInfo, ExtractedPost, JobId, MemberId, WebsiteId};
use crate::domains::crawling::events::{CrawledPageInfo, CrawlEvent, PageExtractionResult};
use crate::domains::crawling::models::{PageSnapshot, PageSummary, WebsiteSnapshot};
use crate::domains::crawling::effects::extraction::{
    hash_content, summarize_pages, synthesize_posts, PageToSummarize, SynthesisInput,
};
use crate::domains::website::models::Website;
use crate::kernel::LinkPriorities;

// =============================================================================
// Static Crawl Keywords (replaces database-stored agent keywords)
// =============================================================================

/// High-priority keywords - pages containing these are crawled first
const HIGH_PRIORITY_KEYWORDS: &[&str] = &[
    // Services pages
    "services",
    "programs",
    "resources",
    "help",
    "assistance",
    "support",
    // Volunteer/donate pages
    "volunteer",
    "donate",
    "give",
    "get-involved",
    "ways-to-help",
    // About/contact pages
    "about",
    "contact",
    "location",
    "hours",
    // Specific services
    "food",
    "housing",
    "legal",
    "immigration",
    "healthcare",
    "employment",
    "education",
    "childcare",
];

/// Skip keywords - pages containing these are not crawled
const SKIP_KEYWORDS: &[&str] = &[
    // Navigation/utility
    "login",
    "signin",
    "signup",
    "register",
    "cart",
    "checkout",
    "account",
    "password",
    "reset",
    // Media/files
    "gallery",
    "photos",
    "videos",
    "downloads",
    "pdf",
    // Policies
    "privacy",
    "terms",
    "cookie",
    "disclaimer",
    // Social/external
    "facebook",
    "twitter",
    "instagram",
    "linkedin",
    "youtube",
    // Other
    "search",
    "sitemap",
    "rss",
    "feed",
    "print",
    "share",
];

/// Build link priorities from static keywords
fn get_crawl_priorities() -> LinkPriorities {
    LinkPriorities {
        high: HIGH_PRIORITY_KEYWORDS.iter().map(|s| s.to_string()).collect(),
        skip: SKIP_KEYWORDS.iter().map(|s| s.to_string()).collect(),
    }
}

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
// Handler: CrawlWebsite
// ============================================================================

async fn handle_crawl_website(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Starting multi-page crawl"
    );

    // Authorization check
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        warn!(
            website_id = %website_id,
            error = %auth_err,
            "Authorization denied for crawl"
        );
        return Ok(CrawlEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CrawlWebsite".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to find website: {}", e),
            });
        }
    };

    // Mark website as crawling
    if let Err(e) = Website::start_crawl(website_id, &ctx.deps().db_pool).await {
        warn!(website_id = %website_id, error = %e, "Failed to update crawl status");
    }

    // Crawl the website
    let max_depth = website.max_crawl_depth;
    let max_pages = website.max_pages_per_crawl.unwrap_or(20);
    let delay = website.crawl_rate_limit_seconds;

    // Use static link priorities
    let priorities = get_crawl_priorities();

    info!(
        website_id = %website_id,
        url = %website.domain,
        max_depth = %max_depth,
        max_pages = %max_pages,
        "Initiating website crawl"
    );

    let crawl_result = match ctx
        .deps()
        .web_scraper
        .crawl(&website.domain, max_depth, max_pages, delay, Some(&priorities))
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Mark crawl as failed
            let _ = Website::complete_crawl(website_id, "failed", 0, &ctx.deps().db_pool).await;
            return Ok(CrawlEvent::WebsiteCrawlFailed {
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
    let mut crawled_pages: Vec<CrawledPageInfo> = Vec::new();

    for page in crawl_result.pages {
        // Create page snapshot
        let (page_snapshot, _is_new) = match PageSnapshot::upsert(
            &ctx.deps().db_pool,
            page.url.clone(),
            page.markdown.clone(), // Use markdown as html
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
        match WebsiteSnapshot::upsert(
            &ctx.deps().db_pool,
            website_id,
            page.url.clone(),
            None, // No specific submitter
        )
        .await
        {
            Ok(website_snapshot) => {
                // Link to page snapshot
                if let Err(e) = website_snapshot
                    .link_snapshot(&ctx.deps().db_pool, page_snapshot.id)
                    .await
                {
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

    // Update website with crawl results
    let _ = Website::complete_crawl(
        website_id,
        "crawling", // Still in progress until extraction
        crawled_pages.len() as i32,
        &ctx.deps().db_pool,
    )
    .await;

    // Collect page snapshot IDs for extraction
    let page_snapshot_ids: Vec<uuid::Uuid> = crawled_pages
        .iter()
        .filter_map(|p| p.snapshot_id)
        .collect();

    info!(
        website_id = %website_id,
        pages_stored = crawled_pages.len(),
        page_snapshots = page_snapshot_ids.len(),
        "Emitting PagesReadyForExtraction event (cross-domain)"
    );

    // Emit integration event for posts domain to handle extraction
    Ok(CrawlEvent::PagesReadyForExtraction {
        website_id,
        job_id,
        page_snapshot_ids,
    })
}

// ============================================================================
// Handler: ExtractPostsFromPages (Two-Pass Extraction)
// ============================================================================

async fn handle_extract_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    pages: Vec<CrawledPageInfo>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        pages_count = pages.len(),
        "Extracting posts using two-pass extraction"
    );

    // Get website for domain info
    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;

    // Build pages to summarize
    let mut pages_to_summarize: Vec<PageToSummarize> = Vec::new();
    let mut snapshot_map: std::collections::HashMap<String, uuid::Uuid> =
        std::collections::HashMap::new();

    for page in &pages {
        let Some(snapshot_id) = page.snapshot_id else {
            warn!(url = %page.url, "No snapshot ID for page, skipping");
            continue;
        };

        let snapshot = match PageSnapshot::find_by_id(&ctx.deps().db_pool, snapshot_id).await {
            Ok(s) => s,
            Err(e) => {
                warn!(snapshot_id = %snapshot_id, error = %e, "Failed to load snapshot");
                continue;
            }
        };

        let raw_content = snapshot.markdown.unwrap_or_else(|| snapshot.html);
        let content_hash = hash_content(&raw_content);

        snapshot_map.insert(page.url.clone(), snapshot_id);
        pages_to_summarize.push(PageToSummarize {
            snapshot_id,
            url: page.url.clone(),
            raw_content,
            content_hash,
        });
    }

    if pages_to_summarize.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: 1,
            pages_crawled: 0,
            should_retry: false,
        });
    }

    // =========================================================================
    // Pass 1: Summarize each page (with caching)
    // =========================================================================
    info!(
        website_id = %website_id,
        pages = pages_to_summarize.len(),
        "Pass 1: Summarizing pages"
    );

    let summaries = match summarize_pages(
        pages_to_summarize,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Pass 1 failed");
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Summarization failed: {}", e),
            });
        }
    };

    if summaries.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: 1,
            pages_crawled: pages.len(),
            should_retry: false,
        });
    }

    // =========================================================================
    // Pass 2: Synthesize posts from all summaries
    // =========================================================================
    info!(
        website_id = %website_id,
        summaries = summaries.len(),
        "Pass 2: Synthesizing posts"
    );

    let extracted_posts = match synthesize_posts(
        SynthesisInput {
            website_domain: website.domain.clone(),
            pages: summaries,
        },
        ctx.deps().ai.as_ref(),
    )
    .await
    {
        Ok(l) => l,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Pass 2 failed");
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Synthesis failed: {}", e),
            });
        }
    };

    // Convert to event format and build page results
    let mut all_posts: Vec<ExtractedPost> = Vec::new();
    let mut page_results: Vec<PageExtractionResult> = Vec::new();

    // Track which pages contributed to posts
    let mut pages_with_posts: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for extracted in &extracted_posts {
        for url in &extracted.source_urls {
            pages_with_posts.insert(url.clone());
        }

        // Convert ExtractedPost from extraction module to common type
        all_posts.push(ExtractedPost {
            title: extracted.title.clone(),
            tldr: extracted.tldr.clone(),
            description: extracted.description.clone(),
            contact: extracted.contact.as_ref().map(|c| ContactInfo {
                phone: c.phone.clone(),
                email: c.email.clone(),
                website: c.website.clone(),
            }),
            location: extracted.location.clone(),
            urgency: Some("normal".to_string()),
            confidence: Some("high".to_string()),
            audience_roles: extracted
                .tags
                .iter()
                .filter(|t| t.kind == "audience_role")
                .map(|t| t.value.clone())
                .collect(),
        });
    }

    // Build page results
    for page in &pages {
        let has_posts = pages_with_posts.contains(&page.url);
        page_results.push(PageExtractionResult {
            url: page.url.clone(),
            snapshot_id: page.snapshot_id,
            listings_count: if has_posts { 1 } else { 0 },
            has_posts,
        });

        // Update page snapshot status
        if let Some(sid) = page.snapshot_id {
            let _ = PageSnapshot::update_extraction_status(
                &ctx.deps().db_pool,
                sid,
                if has_posts { 1 } else { 0 },
                "completed",
            )
            .await;
        }
    }

    // Check if we found any posts
    if all_posts.is_empty() {
        info!(
            website_id = %website_id,
            pages_processed = pages.len(),
            "No posts found in any pages"
        );

        let attempt_count = Website::increment_crawl_attempt(website_id, &ctx.deps().db_pool)
            .await
            .unwrap_or(1);

        let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
        let should_retry = website.should_retry_crawl();

        return Ok(CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: attempt_count,
            pages_crawled: pages.len(),
            should_retry,
        });
    }

    info!(
        website_id = %website_id,
        total_posts = all_posts.len(),
        pages_with_posts = pages_with_posts.len(),
        "Two-pass extraction complete"
    );

    // Reset attempt count on successful extraction
    let _ = Website::reset_crawl_attempts(website_id, &ctx.deps().db_pool).await;

    Ok(CrawlEvent::PostsExtractedFromPages {
        website_id,
        job_id,
        posts: all_posts,
        page_results,
    })
}

// ============================================================================
// Handler: RetryWebsiteCrawl
// ============================================================================

async fn handle_retry_crawl(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        "Retrying website crawl"
    );

    // Reset crawl status to pending for retry
    let _ = sqlx::query(
        "UPDATE websites SET crawl_status = 'pending', updated_at = NOW() WHERE id = $1",
    )
    .bind(website_id.as_uuid())
    .execute(&ctx.deps().db_pool)
    .await;

    // Return a request event to trigger a new crawl
    Ok(CrawlEvent::CrawlWebsiteRequested {
        website_id,
        job_id,
        requested_by: MemberId::new(), // System retry
        is_admin: true,
    })
}

// ============================================================================
// Handler: MarkWebsiteNoPosts
// ============================================================================

async fn handle_mark_no_posts(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        "Marking website as having no posts"
    );

    // Get current attempt count
    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
    let total_attempts = website.crawl_attempt_count.unwrap_or(0);

    // Update status to no_posts_found
    let _ = Website::complete_crawl(
        website_id,
        "no_posts_found",
        website.pages_crawled_count.unwrap_or(0),
        &ctx.deps().db_pool,
    )
    .await;

    Ok(CrawlEvent::WebsiteMarkedNoListings {
        website_id,
        job_id,
        total_attempts,
    })
}

// ============================================================================
// Handler: SyncCrawledPosts
// ============================================================================

async fn handle_sync_crawled_posts(
    website_id: WebsiteId,
    job_id: JobId,
    posts: Vec<ExtractedPost>,
    page_results: Vec<PageExtractionResult>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        posts_count = posts.len(),
        "Syncing crawled posts to database"
    );

    // Use the existing sync logic from posts domain (title-match only, no embedding dedup)
    let sync_result = crate::domains::posts::effects::syncing::sync_extracted_posts(
        website_id,
        posts,
        &ctx.deps().db_pool,
    )
    .await?;

    // Mark crawl as completed
    let _ = Website::complete_crawl(
        website_id,
        "completed",
        page_results.len() as i32,
        &ctx.deps().db_pool,
    )
    .await;

    info!(
        website_id = %website_id,
        new_count = sync_result.new_count,
        updated_count = sync_result.updated_count,
        unchanged_count = sync_result.unchanged_count,
        "Sync completed"
    );

    // Run LLM-based deduplication to identify and merge semantic duplicates
    let dedup_result = llm_deduplicate_website_posts(website_id, ctx).await;
    match dedup_result {
        Ok(deleted_count) => {
            if deleted_count > 0 {
                info!(
                    website_id = %website_id,
                    deleted_count = deleted_count,
                    "LLM deduplication completed"
                );
            }
        }
        Err(e) => {
            warn!(
                website_id = %website_id,
                error = %e,
                "Failed to run LLM deduplication, continuing"
            );
        }
    }

    Ok(CrawlEvent::PostsSynced {
        website_id,
        job_id,
        new_count: sync_result.new_count,
        updated_count: sync_result.updated_count,
        unchanged_count: sync_result.unchanged_count,
    })
}

/// Run LLM-based deduplication for a website's posts
/// Returns the number of duplicate posts soft-deleted
async fn llm_deduplicate_website_posts(
    website_id: WebsiteId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<usize> {
    use crate::domains::posts::effects::deduplication::{deduplicate_posts_llm, apply_dedup_results};

    // Run LLM deduplication analysis
    let dedup_result = deduplicate_posts_llm(
        website_id,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    // Apply the results (soft-delete duplicates)
    let deleted_count = apply_dedup_results(
        dedup_result,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(deleted_count)
}

// ============================================================================
// Handler: RegeneratePosts
// ============================================================================

async fn handle_regenerate_posts(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Regenerating posts from existing snapshots"
    );

    // Authorization check
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        warn!(
            website_id = %website_id,
            error = %auth_err,
            "Authorization denied for regenerate posts"
        );
        return Ok(CrawlEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RegeneratePosts".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to find website: {}", e),
            });
        }
    };

    if website.status != "approved" {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "Website must be approved to regenerate posts".to_string(),
        });
    }

    // Get existing website snapshots
    let snapshots = match WebsiteSnapshot::find_by_website(&ctx.deps().db_pool, website_id).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to load snapshots: {}", e),
            });
        }
    };

    if snapshots.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "No page snapshots found. Run a full crawl first.".to_string(),
        });
    }

    // Build CrawledPageInfo list from existing snapshots
    let crawled_pages: Vec<CrawledPageInfo> = snapshots
        .into_iter()
        .filter_map(|snapshot| {
            snapshot.page_snapshot_id.map(|ps_id| CrawledPageInfo {
                url: snapshot.page_url,
                title: None, // Title will be read from page snapshot during extraction
                snapshot_id: Some(ps_id),
            })
        })
        .collect();

    if crawled_pages.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "No page snapshots with content found. Run a full crawl first.".to_string(),
        });
    }

    info!(
        website_id = %website_id,
        pages_count = crawled_pages.len(),
        "Found existing snapshots, triggering extraction"
    );

    // Return WebsiteCrawled event which will trigger the extraction flow
    Ok(CrawlEvent::WebsiteCrawled {
        website_id,
        job_id,
        pages: crawled_pages,
    })
}

// ============================================================================
// Handler: RegeneratePageSummaries
// ============================================================================

async fn handle_regenerate_page_summaries(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Regenerating page summaries"
    );

    // Authorization check
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        warn!(
            website_id = %website_id,
            error = %auth_err,
            "Authorization denied for regenerate page summaries"
        );
        return Ok(CrawlEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RegeneratePageSummaries".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to find website: {}", e),
            });
        }
    };

    if website.status != "approved" {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "Website must be approved to regenerate page summaries".to_string(),
        });
    }

    // Get existing website snapshots
    let snapshots = match WebsiteSnapshot::find_by_website(&ctx.deps().db_pool, website_id).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to load snapshots: {}", e),
            });
        }
    };

    if snapshots.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "No page snapshots found. Run a full crawl first.".to_string(),
        });
    }

    // Delete existing cached summaries for all page snapshots
    let mut deleted_count = 0;
    for snapshot in &snapshots {
        if let Some(ps_id) = snapshot.page_snapshot_id {
            if let Err(e) = PageSummary::delete_for_snapshot(ps_id, &ctx.deps().db_pool).await {
                warn!(
                    page_snapshot_id = %ps_id,
                    error = %e,
                    "Failed to delete cached summary, continuing"
                );
            } else {
                deleted_count += 1;
            }
        }
    }

    info!(
        website_id = %website_id,
        deleted_count = deleted_count,
        "Deleted cached page summaries"
    );

    // Build pages to summarize
    let mut pages_to_summarize: Vec<PageToSummarize> = Vec::new();

    for snapshot in &snapshots {
        let Some(snapshot_id) = snapshot.page_snapshot_id else {
            continue;
        };

        let page_snapshot = match PageSnapshot::find_by_id(&ctx.deps().db_pool, snapshot_id).await {
            Ok(s) => s,
            Err(e) => {
                warn!(snapshot_id = %snapshot_id, error = %e, "Failed to load snapshot");
                continue;
            }
        };

        let raw_content = page_snapshot
            .markdown
            .unwrap_or_else(|| page_snapshot.html);
        let content_hash = hash_content(&raw_content);

        pages_to_summarize.push(PageToSummarize {
            snapshot_id,
            url: snapshot.page_url.clone(),
            raw_content,
            content_hash,
        });
    }

    if pages_to_summarize.is_empty() {
        return Ok(CrawlEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "No page snapshots with content found.".to_string(),
        });
    }

    info!(
        website_id = %website_id,
        pages = pages_to_summarize.len(),
        "Regenerating page summaries"
    );

    // Run summarization (will create fresh summaries since we deleted the cached ones)
    let summaries = match summarize_pages(
        pages_to_summarize,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Summarization failed");
            return Ok(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Summarization failed: {}", e),
            });
        }
    };

    info!(
        website_id = %website_id,
        summaries_generated = summaries.len(),
        "Page summaries regenerated successfully"
    );

    Ok(CrawlEvent::PageSummariesRegenerated {
        website_id,
        job_id,
        pages_processed: summaries.len(),
    })
}

// ============================================================================
// Handler: RegeneratePageSummary (single page)
// ============================================================================

async fn handle_regenerate_single_page_summary(
    page_snapshot_id: uuid::Uuid,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        page_snapshot_id = %page_snapshot_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Regenerating AI summary for single page"
    );

    // Authorization check
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        warn!(
            page_snapshot_id = %page_snapshot_id,
            error = %auth_err,
            "Authorization denied for regenerate page summary"
        );
        return Ok(CrawlEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RegeneratePageSummary".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get page snapshot
    let page_snapshot = match PageSnapshot::find_by_id(&ctx.deps().db_pool, page_snapshot_id).await
    {
        Ok(s) => s,
        Err(_) => {
            return Ok(CrawlEvent::PageSummaryRegenerated {
                page_snapshot_id,
                job_id,
            });
        }
    };

    // Delete existing cached summary
    if let Err(e) = PageSummary::delete_for_snapshot(page_snapshot_id, &ctx.deps().db_pool).await {
        warn!(
            page_snapshot_id = %page_snapshot_id,
            error = %e,
            "Failed to delete cached summary, continuing"
        );
    }

    // Build page to summarize
    let raw_content = page_snapshot
        .markdown
        .clone()
        .unwrap_or_else(|| page_snapshot.html.clone());
    let content_hash = hash_content(&raw_content);

    let page_to_summarize = PageToSummarize {
        snapshot_id: page_snapshot_id,
        url: page_snapshot.url.clone(),
        raw_content,
        content_hash,
    };

    info!(
        page_snapshot_id = %page_snapshot_id,
        url = %page_snapshot.url,
        "Regenerating AI summary"
    );

    // Run summarization for single page
    let summaries = match summarize_pages(
        vec![page_to_summarize],
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Summarization failed");
            // Return success anyway, as the page still exists
            return Ok(CrawlEvent::PageSummaryRegenerated {
                page_snapshot_id,
                job_id,
            });
        }
    };

    info!(
        page_snapshot_id = %page_snapshot_id,
        success = !summaries.is_empty(),
        "Page summary regenerated"
    );

    Ok(CrawlEvent::PageSummaryRegenerated {
        page_snapshot_id,
        job_id,
    })
}

// ============================================================================
// Handler: RegeneratePagePosts (single page)
// ============================================================================

async fn handle_regenerate_single_page_posts(
    page_snapshot_id: uuid::Uuid,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<CrawlEvent> {
    info!(
        page_snapshot_id = %page_snapshot_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Regenerating posts for single page"
    );

    // Authorization check
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        warn!(
            page_snapshot_id = %page_snapshot_id,
            error = %auth_err,
            "Authorization denied for regenerate page posts"
        );
        return Ok(CrawlEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RegeneratePagePosts".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get page snapshot
    let page_snapshot = match PageSnapshot::find_by_id(&ctx.deps().db_pool, page_snapshot_id).await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Failed to find page snapshot");
            return Ok(CrawlEvent::PagePostsRegenerated {
                page_snapshot_id,
                job_id,
                posts_count: 0,
            });
        }
    };

    // Find the website this page belongs to via website_snapshot
    let website_snapshot =
        match WebsiteSnapshot::find_by_page_snapshot_id(&ctx.deps().db_pool, page_snapshot_id).await
        {
            Ok(Some(ws)) => ws,
            Ok(None) => {
                warn!(
                    page_snapshot_id = %page_snapshot_id,
                    "No website_snapshot found for this page"
                );
                return Ok(CrawlEvent::PagePostsRegenerated {
                    page_snapshot_id,
                    job_id,
                    posts_count: 0,
                });
            }
            Err(e) => {
                warn!(
                    page_snapshot_id = %page_snapshot_id,
                    error = %e,
                    "Failed to find website_snapshot"
                );
                return Ok(CrawlEvent::PagePostsRegenerated {
                    page_snapshot_id,
                    job_id,
                    posts_count: 0,
                });
            }
        };

    let website_id = website_snapshot.get_website_id();

    // Get website for domain info
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Failed to find website");
            return Ok(CrawlEvent::PagePostsRegenerated {
                page_snapshot_id,
                job_id,
                posts_count: 0,
            });
        }
    };

    // Build page info for extraction
    let raw_content = page_snapshot
        .markdown
        .clone()
        .unwrap_or_else(|| page_snapshot.html.clone());
    let content_hash = hash_content(&raw_content);

    let page_to_summarize = PageToSummarize {
        snapshot_id: page_snapshot_id,
        url: page_snapshot.url.clone(),
        raw_content,
        content_hash,
    };

    info!(
        page_snapshot_id = %page_snapshot_id,
        url = %page_snapshot.url,
        "Pass 1: Summarizing page"
    );

    // Pass 1: Summarize the page
    let summaries = match summarize_pages(
        vec![page_to_summarize],
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Summarization failed");
            return Ok(CrawlEvent::PagePostsRegenerated {
                page_snapshot_id,
                job_id,
                posts_count: 0,
            });
        }
    };

    if summaries.is_empty() {
        return Ok(CrawlEvent::PagePostsRegenerated {
            page_snapshot_id,
            job_id,
            posts_count: 0,
        });
    }

    info!(
        page_snapshot_id = %page_snapshot_id,
        "Pass 2: Synthesizing posts"
    );

    // Pass 2: Synthesize posts from summary
    let extracted_posts = match synthesize_posts(
        SynthesisInput {
            website_domain: website.domain.clone(),
            pages: summaries,
        },
        ctx.deps().ai.as_ref(),
    )
    .await
    {
        Ok(l) => l,
        Err(e) => {
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Synthesis failed");
            return Ok(CrawlEvent::PagePostsRegenerated {
                page_snapshot_id,
                job_id,
                posts_count: 0,
            });
        }
    };

    // Convert to common format
    let posts: Vec<ExtractedPost> = extracted_posts
        .into_iter()
        .map(|extracted| ExtractedPost {
            title: extracted.title,
            tldr: extracted.tldr,
            description: extracted.description,
            contact: extracted.contact.map(|c| ContactInfo {
                phone: c.phone,
                email: c.email,
                website: c.website,
            }),
            location: extracted.location,
            urgency: Some("normal".to_string()),
            confidence: Some("high".to_string()),
            audience_roles: extracted
                .tags
                .iter()
                .filter(|t| t.kind == "audience_role")
                .map(|t| t.value.clone())
                .collect(),
        })
        .collect();

    let posts_count = posts.len();

    if posts.is_empty() {
        info!(
            page_snapshot_id = %page_snapshot_id,
            "No posts found in page"
        );
        return Ok(CrawlEvent::PagePostsRegenerated {
            page_snapshot_id,
            job_id,
            posts_count: 0,
        });
    }

    // Sync extracted posts to database (title-match only, no embedding dedup)
    let sync_result =
        match crate::domains::posts::effects::syncing::sync_extracted_posts(
            website_id,
            posts,
            &ctx.deps().db_pool,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Sync failed");
                return Ok(CrawlEvent::PagePostsRegenerated {
                    page_snapshot_id,
                    job_id,
                    posts_count: 0,
                });
            }
        };

    info!(
        page_snapshot_id = %page_snapshot_id,
        new_count = sync_result.new_count,
        updated_count = sync_result.updated_count,
        "Posts regenerated for page"
    );

    // Run LLM deduplication for the website
    if let Err(e) = llm_deduplicate_website_posts(website_id, ctx).await {
        warn!(
            website_id = %website_id,
            error = %e,
            "Failed to run LLM deduplication for page posts"
        );
    }

    Ok(CrawlEvent::PagePostsRegenerated {
        page_snapshot_id,
        job_id,
        posts_count,
    })
}
