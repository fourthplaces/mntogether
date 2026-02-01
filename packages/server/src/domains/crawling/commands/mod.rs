//! Crawling domain commands
//!
//! Commands are requests for IO operations in the crawling workflow.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::crawling::events::CrawledPageInfo;
use crate::common::ExtractedPost;
use crate::domains::crawling::events::PageExtractionResult;

/// Crawling domain commands
/// Following seesaw-rs pattern: Commands are requests for IO operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlCommand {
    // =========================================================================
    // Website Crawling Commands (multi-page crawling workflow)
    // =========================================================================

    /// Crawl a website (multiple pages) using Firecrawl
    CrawlWebsite {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Extract posts from all crawled pages (two-pass extraction)
    ExtractPostsFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
    },

    /// Retry website crawl after no posts found
    RetryWebsiteCrawl {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Mark website as having no posts (terminal state after max retries)
    MarkWebsiteNoPosts {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Sync posts extracted from crawled pages with database
    SyncCrawledPosts {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
        page_results: Vec<PageExtractionResult>,
    },

    /// Regenerate posts from existing page snapshots (skip crawling)
    RegeneratePosts {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Regenerate page summaries for existing snapshots
    RegeneratePageSummaries {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Regenerate AI summary for a single page snapshot
    RegeneratePageSummary {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Regenerate posts for a single page snapshot
    RegeneratePagePosts {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },
}

// Implement Command trait for seesaw-rs integration
impl seesaw_core::Command for CrawlCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        use seesaw_core::ExecutionMode;

        match self {
            // All commands run inline
            Self::CrawlWebsite { .. } => ExecutionMode::Inline,
            Self::ExtractPostsFromPages { .. } => ExecutionMode::Inline,
            Self::RetryWebsiteCrawl { .. } => ExecutionMode::Inline,
            Self::MarkWebsiteNoPosts { .. } => ExecutionMode::Inline,
            Self::SyncCrawledPosts { .. } => ExecutionMode::Inline,
            Self::RegeneratePosts { .. } => ExecutionMode::Inline,
            Self::RegeneratePageSummaries { .. } => ExecutionMode::Inline,
            Self::RegeneratePageSummary { .. } => ExecutionMode::Inline,
            Self::RegeneratePagePosts { .. } => ExecutionMode::Inline,
        }
    }

    fn job_spec(&self) -> Option<seesaw_core::JobSpec> {
        match self {
            Self::CrawlWebsite { website_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "crawl_website",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::ExtractPostsFromPages { website_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_posts_from_pages",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::SyncCrawledPosts { website_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "sync_crawled_posts",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::RegeneratePosts { website_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "regenerate_posts",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::RegeneratePageSummaries { website_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "regenerate_page_summaries",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::RegeneratePageSummary { page_snapshot_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "regenerate_page_summary",
                idempotency_key: Some(page_snapshot_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::RegeneratePagePosts { page_snapshot_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "regenerate_page_posts",
                idempotency_key: Some(page_snapshot_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            // Inline commands don't need job specs
            _ => None,
        }
    }

    fn serialize_to_json(&self) -> Option<JsonValue> {
        serde_json::to_value(self).ok()
    }
}
