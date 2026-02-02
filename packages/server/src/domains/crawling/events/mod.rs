//! Crawling domain events
//!
//! Events are immutable facts that occurred during the crawling workflow.
//!
//! Architecture (seesaw 0.3.0):
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...

use serde::{Deserialize, Serialize};

use crate::common::{ExtractedPost, JobId, MemberId, WebsiteId};

/// Information about a crawled page (used in WebsiteCrawled event)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPageInfo {
    pub url: String,
    pub title: Option<String>,
    pub snapshot_id: Option<uuid::Uuid>,
}

/// Result of extracting posts from a single page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageExtractionResult {
    pub url: String,
    pub snapshot_id: Option<uuid::Uuid>,
    pub listings_count: usize,
    pub has_posts: bool,
}

/// Crawling domain events
/// Following seesaw 0.3.0 pattern: Request events → Effect → Fact events → Internal edges
#[derive(Debug, Clone)]
pub enum CrawlEvent {
    // =========================================================================
    // Request Events (from edges and internal edges)
    // =========================================================================

    /// Admin requests to crawl a website (multi-page)
    CrawlWebsiteRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Request to extract posts from crawled pages (triggered internally)
    ExtractPostsFromPagesRequested {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
    },

    /// Request to sync extracted posts to database (triggered by internal edge)
    SyncCrawledPostsRequested {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
        page_results: Vec<PageExtractionResult>,
    },

    /// Request to retry crawl after no posts found (triggered by internal edge)
    RetryWebsiteCrawlRequested {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Request to mark website as having no posts (triggered by internal edge)
    MarkWebsiteNoPostsRequested {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Admin requests to regenerate posts from existing page snapshots
    RegeneratePostsRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin requests to regenerate page summaries for existing snapshots
    RegeneratePageSummariesRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin requests to regenerate AI summary for a single page snapshot
    RegeneratePageSummaryRequested {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin requests to regenerate posts for a single page snapshot
    RegeneratePagePostsRequested {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    // =========================================================================
    // Fact Events (from effects - what actually happened)
    // =========================================================================

    /// Website was crawled (multiple pages discovered)
    WebsiteCrawled {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
    },

    /// Integration event: Pages crawled, ready for post extraction
    /// Posts domain listens for this and handles extraction
    /// NOTE: Carries IDs only, not payloads - posts domain fetches what it needs
    PagesReadyForExtraction {
        website_id: WebsiteId,
        job_id: JobId,
        page_snapshot_ids: Vec<uuid::Uuid>,
    },

    /// No posts found after crawling all pages
    WebsiteCrawlNoListings {
        website_id: WebsiteId,
        job_id: JobId,
        attempt_number: i32,
        pages_crawled: usize,
        should_retry: bool,
    },

    /// Terminal: website marked as having no posts after max retries
    WebsiteMarkedNoListings {
        website_id: WebsiteId,
        job_id: JobId,
        total_attempts: i32,
    },

    /// Website crawl failed
    WebsiteCrawlFailed {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    /// Posts extracted from multiple crawled pages
    PostsExtractedFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
        page_results: Vec<PageExtractionResult>,
    },

    /// Posts were synced with database (from crawled pages)
    PostsSynced {
        website_id: WebsiteId,
        job_id: JobId,
        new_count: usize,
        updated_count: usize,
        unchanged_count: usize,
    },

    /// Page summaries regenerated successfully
    PageSummariesRegenerated {
        website_id: WebsiteId,
        job_id: JobId,
        pages_processed: usize,
    },

    /// Single page summary regenerated successfully
    PageSummaryRegenerated {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
    },

    /// Single page posts regenerated successfully
    PagePostsRegenerated {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        posts_count: usize,
    },

    // =========================================================================
    // Authorization Events
    // =========================================================================

    /// User attempted admin action without permission
    AuthorizationDenied {
        user_id: MemberId,
        action: String,
        reason: String,
    },
}
