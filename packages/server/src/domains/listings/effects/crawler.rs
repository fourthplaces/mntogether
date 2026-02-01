// CrawlerEffect - Handles multi-page website crawling workflow
//
// This effect handles:
// - CrawlWebsite: Crawl multiple pages from a website
// - ExtractListingsFromPages: Extract listings from all crawled pages
// - RetryWebsiteCrawl: Retry crawl after no listings found
// - MarkWebsiteNoListings: Mark website as having no listings (terminal)
// - SyncCrawledListings: Sync extracted listings to database

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::{info, warn};

use super::deps::ServerDeps;
use super::extraction::{
    summarize::{hash_content, summarize_pages},
    synthesize::synthesize_listings,
    types::{PageToSummarize, SynthesisInput},
};
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{ContactInfo, ExtractedListing, JobId, MemberId, WebsiteId};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::{CrawledPageInfo, ListingEvent, PageExtractionResult};
use crate::domains::scraping::models::{PageSnapshot, Website, WebsiteSnapshot};
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

/// Crawler Effect - Handles multi-page website crawling
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct CrawlerEffect;

#[async_trait]
impl Effect<ListingCommand, ServerDeps> for CrawlerEffect {
    type Event = ListingEvent;

    async fn execute(
        &self,
        cmd: ListingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ListingEvent> {
        match cmd {
            ListingCommand::CrawlWebsite {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_crawl_website(website_id, job_id, requested_by, is_admin, &ctx).await,

            ListingCommand::ExtractListingsFromPages {
                website_id,
                job_id,
                pages,
            } => handle_extract_from_pages(website_id, job_id, pages, &ctx).await,

            ListingCommand::RetryWebsiteCrawl { website_id, job_id } => {
                handle_retry_crawl(website_id, job_id, &ctx).await
            }

            ListingCommand::MarkWebsiteNoListings { website_id, job_id } => {
                handle_mark_no_listings(website_id, job_id, &ctx).await
            }

            ListingCommand::SyncCrawledListings {
                website_id,
                job_id,
                listings,
                page_results,
            } => handle_sync_crawled_listings(website_id, job_id, listings, page_results, &ctx).await,

            _ => anyhow::bail!("CrawlerEffect: Unexpected command"),
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
) -> Result<ListingEvent> {
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
        return Ok(ListingEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CrawlWebsite".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(ListingEvent::WebsiteCrawlFailed {
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
            return Ok(ListingEvent::WebsiteCrawlFailed {
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

    info!(
        website_id = %website_id,
        pages_stored = crawled_pages.len(),
        "Emitting WebsiteCrawled event"
    );

    Ok(ListingEvent::WebsiteCrawled {
        website_id,
        job_id,
        pages: crawled_pages,
    })
}

// ============================================================================
// Handler: ExtractListingsFromPages (Two-Pass Extraction)
// ============================================================================

async fn handle_extract_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    pages: Vec<CrawledPageInfo>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        pages_count = pages.len(),
        "Extracting listings using two-pass extraction"
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
        return Ok(ListingEvent::WebsiteCrawlNoListings {
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

    let summaries: Vec<super::extraction::types::SummarizedPage> = match summarize_pages(
        pages_to_summarize,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Pass 1 failed");
            return Ok(ListingEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Summarization failed: {}", e),
            });
        }
    };

    if summaries.is_empty() {
        return Ok(ListingEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: 1,
            pages_crawled: pages.len(),
            should_retry: false,
        });
    }

    // =========================================================================
    // Pass 2: Synthesize listings from all summaries
    // =========================================================================
    info!(
        website_id = %website_id,
        summaries = summaries.len(),
        "Pass 2: Synthesizing listings"
    );

    let extracted_listings = match synthesize_listings(
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
            return Ok(ListingEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Synthesis failed: {}", e),
            });
        }
    };

    // Convert to event format and build page results
    let mut all_listings: Vec<ExtractedListing> = Vec::new();
    let mut page_results: Vec<PageExtractionResult> = Vec::new();

    // Track which pages contributed to listings
    let mut pages_with_listings: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for listing in &extracted_listings {
        for url in &listing.source_urls {
            pages_with_listings.insert(url.clone());
        }

        // Convert ExtractedListing from extraction module to common type
        all_listings.push(ExtractedListing {
            title: listing.title.clone(),
            tldr: listing.tldr.clone(),
            description: listing.description.clone(),
            contact: listing.contact.as_ref().map(|c| ContactInfo {
                phone: c.phone.clone(),
                email: c.email.clone(),
                website: c.website.clone(),
            }),
            urgency: Some("normal".to_string()),
            confidence: Some("high".to_string()),
            audience_roles: listing
                .tags
                .iter()
                .filter(|t| t.kind == "audience_role")
                .map(|t| t.value.clone())
                .collect(),
        });
    }

    // Build page results
    for page in &pages {
        let has_listings = pages_with_listings.contains(&page.url);
        page_results.push(PageExtractionResult {
            url: page.url.clone(),
            snapshot_id: page.snapshot_id,
            listings_count: if has_listings { 1 } else { 0 },
            has_listings,
        });

        // Update page snapshot status
        if let Some(sid) = page.snapshot_id {
            let _ = PageSnapshot::update_extraction_status(
                &ctx.deps().db_pool,
                sid,
                if has_listings { 1 } else { 0 },
                "completed",
            )
            .await;
        }
    }

    // Check if we found any listings
    if all_listings.is_empty() {
        info!(
            website_id = %website_id,
            pages_processed = pages.len(),
            "No listings found in any pages"
        );

        let attempt_count = Website::increment_crawl_attempt(website_id, &ctx.deps().db_pool)
            .await
            .unwrap_or(1);

        let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
        let should_retry = website.should_retry_crawl();

        return Ok(ListingEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: attempt_count,
            pages_crawled: pages.len(),
            should_retry,
        });
    }

    info!(
        website_id = %website_id,
        total_listings = all_listings.len(),
        pages_with_listings = pages_with_listings.len(),
        "Two-pass extraction complete"
    );

    // Reset attempt count on successful extraction
    let _ = Website::reset_crawl_attempts(website_id, &ctx.deps().db_pool).await;

    Ok(ListingEvent::ListingsExtractedFromPages {
        website_id,
        job_id,
        listings: all_listings,
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
) -> Result<ListingEvent> {
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

    // Return the same event type that triggers crawl
    // The state machine will route this appropriately
    Ok(ListingEvent::ScrapeSourceRequested {
        source_id: website_id,
        job_id,
        requested_by: MemberId::new(), // System retry
        is_admin: true,
    })
}

// ============================================================================
// Handler: MarkWebsiteNoListings
// ============================================================================

async fn handle_mark_no_listings(
    website_id: WebsiteId,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        "Marking website as having no listings"
    );

    // Get current attempt count
    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
    let total_attempts = website.crawl_attempt_count.unwrap_or(0);

    // Update status to no_listings_found
    let _ = Website::complete_crawl(
        website_id,
        "no_listings_found",
        website.pages_crawled_count.unwrap_or(0),
        &ctx.deps().db_pool,
    )
    .await;

    Ok(ListingEvent::WebsiteMarkedNoListings {
        website_id,
        job_id,
        total_attempts,
    })
}

// ============================================================================
// Handler: SyncCrawledListings
// ============================================================================

async fn handle_sync_crawled_listings(
    website_id: WebsiteId,
    job_id: JobId,
    listings: Vec<ExtractedListing>,
    page_results: Vec<PageExtractionResult>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        listings_count = listings.len(),
        "Syncing crawled listings to database"
    );

    // Use the existing sync logic from the syncing module
    let sync_result = super::syncing::sync_extracted_listings(
        website_id,
        listings,
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
        changed_count = sync_result.changed_count,
        disappeared_count = sync_result.disappeared_count,
        "Sync completed"
    );

    Ok(ListingEvent::ListingsSynced {
        source_id: website_id,
        job_id,
        new_count: sync_result.new_count,
        changed_count: sync_result.changed_count,
        disappeared_count: sync_result.disappeared_count,
    })
}
