//! Crawling domain events
//!
//! Events are immutable facts that occurred during the crawling workflow.
//!
//! Architecture (direct-call pattern):
//!   GraphQL → Action (via process) → emit Fact Event → Cascade Effect → Handler
//!
//! ALL *Requested events have been removed. GraphQL calls actions directly.
//! Effects watch FACT events and call handlers directly for cascading.

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

/// Crawling domain events - FACT EVENTS ONLY
///
/// These are immutable facts about what happened. Effects watch these
/// and call handlers directly for cascade workflows (no *Requested events).
#[derive(Debug, Clone)]
pub enum CrawlEvent {
    // =========================================================================
    // Fact Events (emitted by actions - what actually happened)
    // =========================================================================
    /// Website was crawled (multiple pages discovered)
    WebsiteCrawled {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
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
