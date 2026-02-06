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
use uuid::Uuid;

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
/// ## Event-Driven Pipeline
///
/// - `WebsiteIngested | WebsitePostsRegenerated | WebsitePagesDiscovered` → ExtractPostsJob → PostsExtractedFromPages
/// - `PostsExtractedFromPages` → SyncPostsJob → `PostsSynced`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlEvent {
    // =========================================================================
    // Ingestion & Discovery Events (trigger post extraction cascade)
    // =========================================================================
    /// Website ingested via extraction library
    ///
    /// Emitted by `ingest_website()` after pages are crawled and summarized.
    /// Triggers: ExtractPostsJob → SyncPostsJob pipeline
    WebsiteIngested {
        website_id: WebsiteId,
        job_id: JobId,
        pages_crawled: usize,
        pages_summarized: usize,
    },

    /// Website posts regenerated from existing pages
    ///
    /// Emitted by `regenerate_posts()` when regenerating from existing page_snapshots.
    /// Triggers: ExtractPostsJob → SyncPostsJob pipeline
    WebsitePostsRegenerated {
        website_id: WebsiteId,
        job_id: JobId,
        pages_processed: usize,
    },

    /// Pages discovered via search (Tavily) and stored
    ///
    /// Emitted by `discover_website()` after pages are discovered via search.
    /// Triggers: ExtractPostsJob → SyncPostsJob pipeline
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
    /// Emitted by ExtractPostsJob after agentic extraction.
    /// Triggers: SyncPostsJob
    PostsExtractedFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
        page_results: Vec<PageExtractionResult>,
    },

    /// Posts synced to database
    ///
    /// Terminal event - emitted by SyncPostsJob.
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
    /// Triggers: Website marking (no further cascade)
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
    // Job Replacement Events (queued effects pick these up)
    // =========================================================================
    /// Crawl website work enqueued
    ///
    /// Emitted by GraphQL mutation. Picked up by `crawl_website_effect`.
    CrawlWebsiteEnqueued {
        website_id: Uuid,
        visitor_id: Uuid,
        use_firecrawl: bool,
    },

    /// Post extraction work enqueued
    ///
    /// Emitted after crawl/regeneration completes. Picked up by `extract_posts_effect`.
    PostsExtractionEnqueued { website_id: Uuid },

    /// Post sync work enqueued
    ///
    /// Emitted after extraction completes with posts. Picked up by `sync_posts_effect`.
    /// Always uses LLM sync to stage proposals for human review.
    PostsSyncEnqueued {
        website_id: WebsiteId,
        posts: Vec<ExtractedPost>,
    },

    /// Post regeneration work enqueued
    ///
    /// Emitted by GraphQL mutation. Picked up by `regenerate_posts_effect`.
    PostsRegenerationEnqueued { website_id: Uuid, visitor_id: Uuid },

    /// Single post regeneration work enqueued
    ///
    /// Emitted by GraphQL mutation. Picked up by `regenerate_single_post_effect`.
    SinglePostRegenerationEnqueued { post_id: Uuid },

    // =========================================================================
    // Fan-out Investigation Events (batch/join pipeline)
    // =========================================================================
    /// Single post investigation enqueued (one per deduplicated narrative)
    ///
    /// Emitted as a batch by `extract_narratives` effect.
    /// Picked up by `investigate_post` effect (parallel per post).
    PostInvestigationEnqueued {
        website_id: WebsiteId,
        title: String,
        tldr: String,
        description: String,
        source_url: String,
    },

    /// Single post investigation completed
    ///
    /// Emitted by `investigate_post` effect after AI investigation.
    /// Joined by `join_investigations` effect.
    PostInvestigated {
        website_id: WebsiteId,
        post: ExtractedPost,
    },
}
