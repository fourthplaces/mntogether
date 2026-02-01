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
use crate::common::{ContactInfo, ExtractedPost, JobId, MemberId, WebsiteId};
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::{CrawledPageInfo, PostEvent, PageExtractionResult};
use crate::domains::posts::models::post::Post;
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
impl Effect<PostCommand, ServerDeps> for CrawlerEffect {
    type Event = PostEvent;

    async fn execute(
        &self,
        cmd: PostCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<PostEvent> {
        match cmd {
            PostCommand::CrawlWebsite {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_crawl_website(website_id, job_id, requested_by, is_admin, &ctx).await,

            PostCommand::ExtractListingsFromPages {
                website_id,
                job_id,
                pages,
            } => handle_extract_from_pages(website_id, job_id, pages, &ctx).await,

            PostCommand::RetryWebsiteCrawl { website_id, job_id } => {
                handle_retry_crawl(website_id, job_id, &ctx).await
            }

            PostCommand::MarkWebsiteNoListings { website_id, job_id } => {
                handle_mark_no_listings(website_id, job_id, &ctx).await
            }

            PostCommand::SyncCrawledListings {
                website_id,
                job_id,
                listings,
                page_results,
            } => handle_sync_crawled_listings(website_id, job_id, listings, page_results, &ctx).await,

            PostCommand::RegeneratePosts {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => handle_regenerate_posts(website_id, job_id, requested_by, is_admin, &ctx).await,

            PostCommand::RegeneratePageSummaries {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                handle_regenerate_page_summaries(website_id, job_id, requested_by, is_admin, &ctx)
                    .await
            }

            PostCommand::RegeneratePageSummary {
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
            }

            PostCommand::RegeneratePagePosts {
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
            }

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
) -> Result<PostEvent> {
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
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CrawlWebsite".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(PostEvent::WebsiteCrawlFailed {
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
            return Ok(PostEvent::WebsiteCrawlFailed {
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

    Ok(PostEvent::WebsiteCrawled {
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
) -> Result<PostEvent> {
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
        return Ok(PostEvent::WebsiteCrawlNoListings {
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
            return Ok(PostEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Summarization failed: {}", e),
            });
        }
    };

    if summaries.is_empty() {
        return Ok(PostEvent::WebsiteCrawlNoListings {
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
            return Ok(PostEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Synthesis failed: {}", e),
            });
        }
    };

    // Convert to event format and build page results
    let mut all_listings: Vec<ExtractedPost> = Vec::new();
    let mut page_results: Vec<PageExtractionResult> = Vec::new();

    // Track which pages contributed to listings
    let mut pages_with_listings: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for extracted in &extracted_listings {
        for url in &extracted.source_urls {
            pages_with_listings.insert(url.clone());
        }

        // Convert ExtractedPost from extraction module to common type
        all_listings.push(ExtractedPost {
            title: extracted.title.clone(),
            tldr: extracted.tldr.clone(),
            description: extracted.description.clone(),
            contact: extracted.contact.as_ref().map(|c| ContactInfo {
                phone: c.phone.clone(),
                email: c.email.clone(),
                website: c.website.clone(),
            }),
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

        return Ok(PostEvent::WebsiteCrawlNoListings {
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

    Ok(PostEvent::ListingsExtractedFromPages {
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
) -> Result<PostEvent> {
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
    Ok(PostEvent::ScrapeSourceRequested {
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
) -> Result<PostEvent> {
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

    Ok(PostEvent::WebsiteMarkedNoListings {
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
    listings: Vec<ExtractedPost>,
    page_results: Vec<PageExtractionResult>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<PostEvent> {
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

    // Generate embeddings for listings that don't have them
    let embeddings_result = generate_missing_embeddings_for_website(website_id, ctx).await;
    match embeddings_result {
        Ok((processed, failed)) => {
            info!(
                website_id = %website_id,
                processed = processed,
                failed = failed,
                "Embedding generation completed"
            );
        }
        Err(e) => {
            warn!(
                website_id = %website_id,
                error = %e,
                "Failed to generate some embeddings, continuing"
            );
        }
    }

    Ok(PostEvent::ListingsSynced {
        source_id: website_id,
        job_id,
        new_count: sync_result.new_count,
        changed_count: sync_result.changed_count,
        disappeared_count: sync_result.disappeared_count,
    })
}

/// Generate embeddings for listings in a website that don't have them yet
async fn generate_missing_embeddings_for_website(
    website_id: WebsiteId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<(i32, i32)> {
    // Find listings without embeddings for this website (process up to 100 at a time)
    let listings = Post::find_without_embeddings_for_website(
        website_id,
        100,
        &ctx.deps().db_pool,
    )
    .await?;

    if listings.is_empty() {
        return Ok((0, 0));
    }

    info!(
        website_id = %website_id,
        count = listings.len(),
        "Generating embeddings for listings"
    );

    let mut processed = 0;
    let mut failed = 0;

    for listing in &listings {
        // Build embedding content from listing fields
        let content_for_embedding = format!(
            "{}\n\n{}\n\nTL;DR: {}\nOrganization: {}",
            listing.title,
            listing.description,
            listing.tldr.as_deref().unwrap_or(""),
            listing.organization_name
        );

        match ctx.deps().embedding_service.generate(&content_for_embedding).await {
            Ok(embedding) => {
                if let Err(e) = Post::update_embedding(listing.id, &embedding, &ctx.deps().db_pool).await {
                    warn!(
                        post_id = %listing.id.as_uuid(),
                        error = %e,
                        "Failed to save embedding"
                    );
                    failed += 1;
                } else {
                    processed += 1;
                }
            }
            Err(e) => {
                warn!(
                    post_id = %listing.id.as_uuid(),
                    error = %e,
                    "Failed to generate embedding"
                );
                failed += 1;
            }
        }
    }

    Ok((processed, failed))
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
) -> Result<PostEvent> {
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
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RegeneratePosts".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(PostEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to find website: {}", e),
            });
        }
    };

    if website.status != "approved" {
        return Ok(PostEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "Website must be approved to regenerate posts".to_string(),
        });
    }

    // Get existing website snapshots
    let snapshots = match WebsiteSnapshot::find_by_website(&ctx.deps().db_pool, website_id).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(PostEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to load snapshots: {}", e),
            });
        }
    };

    if snapshots.is_empty() {
        return Ok(PostEvent::WebsiteCrawlFailed {
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
        return Ok(PostEvent::WebsiteCrawlFailed {
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
    Ok(PostEvent::WebsiteCrawled {
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
) -> Result<PostEvent> {
    use crate::domains::scraping::models::PageSummary;

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
        return Ok(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RegeneratePageSummaries".to_string(),
            reason: auth_err.to_string(),
        });
    }

    // Get website from database
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            return Ok(PostEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to find website: {}", e),
            });
        }
    };

    if website.status != "approved" {
        return Ok(PostEvent::WebsiteCrawlFailed {
            website_id,
            job_id,
            reason: "Website must be approved to regenerate page summaries".to_string(),
        });
    }

    // Get existing website snapshots
    let snapshots = match WebsiteSnapshot::find_by_website(&ctx.deps().db_pool, website_id).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(PostEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Failed to load snapshots: {}", e),
            });
        }
    };

    if snapshots.is_empty() {
        return Ok(PostEvent::WebsiteCrawlFailed {
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
        return Ok(PostEvent::WebsiteCrawlFailed {
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
            return Ok(PostEvent::WebsiteCrawlFailed {
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

    Ok(PostEvent::PageSummariesRegenerated {
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
) -> Result<PostEvent> {
    use crate::domains::scraping::models::PageSummary;

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
        return Ok(PostEvent::AuthorizationDenied {
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
            return Ok(PostEvent::PageSummaryRegenerated {
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
            return Ok(PostEvent::PageSummaryRegenerated {
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

    Ok(PostEvent::PageSummaryRegenerated {
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
) -> Result<PostEvent> {
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
        return Ok(PostEvent::AuthorizationDenied {
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
            return Ok(PostEvent::PagePostsRegenerated {
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
                return Ok(PostEvent::PagePostsRegenerated {
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
                return Ok(PostEvent::PagePostsRegenerated {
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
            return Ok(PostEvent::PagePostsRegenerated {
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
            return Ok(PostEvent::PagePostsRegenerated {
                page_snapshot_id,
                job_id,
                posts_count: 0,
            });
        }
    };

    if summaries.is_empty() {
        return Ok(PostEvent::PagePostsRegenerated {
            page_snapshot_id,
            job_id,
            posts_count: 0,
        });
    }

    info!(
        page_snapshot_id = %page_snapshot_id,
        "Pass 2: Synthesizing listings"
    );

    // Pass 2: Synthesize listings from summary
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
            warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Synthesis failed");
            return Ok(PostEvent::PagePostsRegenerated {
                page_snapshot_id,
                job_id,
                posts_count: 0,
            });
        }
    };

    // Convert to common format
    let listings: Vec<crate::common::ExtractedPost> = extracted_listings
        .into_iter()
        .map(|extracted| crate::common::ExtractedPost {
            title: extracted.title,
            tldr: extracted.tldr,
            description: extracted.description,
            contact: extracted.contact.map(|c| crate::common::ContactInfo {
                phone: c.phone,
                email: c.email,
                website: c.website,
            }),
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

    let posts_count = listings.len();

    if listings.is_empty() {
        info!(
            page_snapshot_id = %page_snapshot_id,
            "No listings found in page"
        );
        return Ok(PostEvent::PagePostsRegenerated {
            page_snapshot_id,
            job_id,
            posts_count: 0,
        });
    }

    // Sync extracted listings to database
    let sync_result =
        match super::syncing::sync_extracted_listings(website_id, listings, &ctx.deps().db_pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(page_snapshot_id = %page_snapshot_id, error = %e, "Sync failed");
                return Ok(PostEvent::PagePostsRegenerated {
                    page_snapshot_id,
                    job_id,
                    posts_count: 0,
                });
            }
        };

    info!(
        page_snapshot_id = %page_snapshot_id,
        new_count = sync_result.new_count,
        changed_count = sync_result.changed_count,
        "Posts regenerated for page"
    );

    // Generate embeddings for any new listings that don't have them
    if let Err(e) = generate_missing_embeddings_for_website(website_id, ctx).await {
        warn!(
            website_id = %website_id,
            error = %e,
            "Failed to generate embeddings for page posts"
        );
    }

    Ok(PostEvent::PagePostsRegenerated {
        page_snapshot_id,
        job_id,
        posts_count,
    })
}
