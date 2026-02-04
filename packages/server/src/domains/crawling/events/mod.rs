//! Crawling domain events
//!
//! Events are immutable facts that occurred during the crawling workflow.
//!
//! Architecture (direct-call pattern):
//!   GraphQL → Action (via process) → emit Fact Event → Cascade Effect → Handler
//!
//! ALL *Requested events have been removed. GraphQL calls actions directly.
//! Effects watch FACT events and call handlers directly for cascading.
//!
//! ## PLATINUM RULE: Events Are Facts Only
//!
//! Events represent facts about what happened - never errors or failures.
//! Errors are returned via Result::Err, not as events.

use serde::{Deserialize, Serialize};

use crate::common::{ExtractedPost, JobId, WebsiteId};

/// Information about a crawled/discovered page
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
/// and call handlers directly for cascade workflows.
///
/// ## Key Cascades
///
/// - `WebsiteIngested` → `handle_extract_posts_from_pages` → `PostsExtractedFromPages`
/// - `WebsitePostsRegenerated` → `handle_extract_posts_from_pages` → `PostsExtractedFromPages`
/// - `WebsitePagesDiscovered` → `handle_extract_posts_from_pages` → `PostsExtractedFromPages`
/// - `PostsExtractedFromPages` → `handle_sync_crawled_posts` → `PostsSynced`
#[derive(Debug, Clone)]
pub enum CrawlEvent {
    // =========================================================================
    // Ingestion & Discovery Events (trigger post extraction cascade)
    // =========================================================================
    /// Website ingested via extraction library
    ///
    /// Emitted by `ingest_website()` after pages are crawled and summarized.
    /// Cascades to: `handle_extract_posts_from_pages`
    WebsiteIngested {
        website_id: WebsiteId,
        job_id: JobId,
        pages_crawled: usize,
        pages_summarized: usize,
    },

    /// Website posts regenerated from existing pages
    ///
    /// Emitted by `regenerate_posts()` when regenerating from existing page_snapshots.
    /// Cascades to: `handle_extract_posts_from_pages`
    WebsitePostsRegenerated {
        website_id: WebsiteId,
        job_id: JobId,
        pages_processed: usize,
    },

    /// Pages discovered via search (Tavily) and stored
    ///
    /// Emitted by `discover_website()` after pages are discovered via search.
    /// Cascades to: `handle_extract_posts_from_pages`
    WebsitePagesDiscovered {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
        discovery_method: String, // "tavily", "sitemap", etc.
    },

    // =========================================================================
    // Extraction & Sync Events
    // =========================================================================
    /// Posts extracted from crawled/ingested pages
    ///
    /// Emitted by `handle_extract_posts_from_pages` after agentic extraction.
    /// Cascades to: `handle_sync_crawled_posts`
    PostsExtractedFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
        page_results: Vec<PageExtractionResult>,
    },

    /// Posts synced to database
    ///
    /// Terminal event - emitted by `handle_sync_crawled_posts`.
    PostsSynced {
        website_id: WebsiteId,
        job_id: JobId,
        new_count: usize,
        updated_count: usize,
        unchanged_count: usize,
    },

    // =========================================================================
    // No-Posts Events
    // =========================================================================
    /// No posts found after crawling all pages
    ///
    /// Cascades to: `handle_mark_no_posts`
    WebsiteCrawlNoListings {
        website_id: WebsiteId,
        job_id: JobId,
        attempt_number: i32,
        pages_crawled: usize,
    },

    /// Terminal: website marked as having no posts
    WebsiteMarkedNoListings {
        website_id: WebsiteId,
        job_id: JobId,
        total_attempts: i32,
    },

    // =========================================================================
    // Page-Level Events (terminal)
    // =========================================================================
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
}
