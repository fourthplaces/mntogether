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
use super::listing::extract_domain;
use super::listing_extraction;
use crate::common::auth::{Actor, AdminCapability};
use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::{
    CrawledPageInfo, ExtractedListing, ListingEvent, PageExtractionResult,
};
use crate::domains::scraping::models::{PageSnapshot, Website, WebsiteSnapshot};

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

    // Crawl the website using Firecrawl
    let max_depth = website.max_crawl_depth;
    let max_pages = website.max_pages_per_crawl.unwrap_or(20);
    let delay = website.crawl_rate_limit_seconds;

    info!(
        website_id = %website_id,
        url = %website.domain,
        max_depth = %max_depth,
        max_pages = %max_pages,
        "Initiating Firecrawl crawl"
    );

    let crawl_result = match ctx
        .deps()
        .web_scraper
        .crawl(&website.domain, max_depth, max_pages, delay)
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
            "firecrawl_crawl".to_string(),
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
// Handler: ExtractListingsFromPages
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
        "Extracting listings from crawled pages"
    );

    // Get website for organization name
    let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
    let organization_name =
        extract_domain(&website.domain).unwrap_or_else(|| "Unknown Organization".to_string());

    let mut all_listings: Vec<ExtractedListing> = Vec::new();
    let mut page_results: Vec<PageExtractionResult> = Vec::new();

    for page in &pages {
        // Get page content from snapshot
        let content = if let Some(snapshot_id) = page.snapshot_id {
            match PageSnapshot::find_by_id(&ctx.deps().db_pool, snapshot_id).await {
                Ok(snapshot) => snapshot.markdown.unwrap_or_else(|| snapshot.html),
                Err(e) => {
                    warn!(snapshot_id = %snapshot_id, error = %e, "Failed to load page snapshot");
                    continue;
                }
            }
        } else {
            warn!(url = %page.url, "No snapshot ID for page");
            continue;
        };

        // Extract listings from this page with PII scrubbing
        info!(url = %page.url, content_length = content.len(), "Extracting listings from page");

        let listings = match listing_extraction::extract_listings_with_pii_scrub(
            ctx.deps().ai.as_ref(),
            ctx.deps().pii_detector.as_ref(),
            &organization_name,
            &content,
            &page.url,
        )
        .await
        {
            Ok(l) => l,
            Err(e) => {
                warn!(url = %page.url, error = %e, "Failed to extract listings from page");
                page_results.push(PageExtractionResult {
                    url: page.url.clone(),
                    snapshot_id: page.snapshot_id,
                    listings_count: 0,
                    has_listings: false,
                });
                continue;
            }
        };

        let listings_count = listings.len();
        let has_listings = listings_count > 0;

        info!(
            url = %page.url,
            listings_count = listings_count,
            "Extracted listings from page"
        );

        // Update page snapshot with extraction results
        if let Some(snapshot_id) = page.snapshot_id {
            let _ = PageSnapshot::update_extraction_status(
                &ctx.deps().db_pool,
                snapshot_id,
                listings_count as i32,
                "completed",
            )
            .await;
        }

        page_results.push(PageExtractionResult {
            url: page.url.clone(),
            snapshot_id: page.snapshot_id,
            listings_count,
            has_listings,
        });

        all_listings.extend(listings);
    }

    // Check if we found any listings
    if all_listings.is_empty() {
        info!(
            website_id = %website_id,
            pages_processed = pages.len(),
            "No listings found in any pages"
        );

        // Increment attempt count
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
        pages_with_listings = page_results.iter().filter(|p| p.has_listings).count(),
        "Listings extraction complete"
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
