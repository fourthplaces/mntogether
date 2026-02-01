use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::common::{JobId, PostId, MemberId, WebsiteId};
use crate::domains::posts::events::{CrawledPageInfo, ExtractedPost};
use crate::domains::posts::models::post_report::PostReportId;

/// Listings domain commands
/// Following seesaw-rs pattern: Commands are requests for IO operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostCommand {
    /// Scrape a source URL using Firecrawl
    ScrapeSource {
        source_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Create organization source from user-submitted link
    CreateWebsiteFromLink {
        url: String,
        organization_name: String,
        submitter_contact: Option<String>,
    },

    /// Scrape a user-submitted resource link (public submission)
    ScrapeResourceLink {
        job_id: JobId,
        url: String,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Extract listings from scraped content using AI
    ExtractPosts {
        source_id: WebsiteId,
        job_id: JobId,
        organization_name: String,
        content: String,
    },

    /// Extract listings from user-submitted resource link
    ExtractPostsFromResourceLink {
        job_id: JobId,
        url: String,
        content: String,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Sync extracted listings with database
    SyncPosts {
        source_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
    },

    /// Create a new listing (from user submission)
    CreatePostEntry {
        member_id: Option<MemberId>,
        organization_name: String,
        title: String,
        description: String,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        ip_address: Option<String>, // Converted from IpAddr before storing
        submission_type: String,    // 'user_submitted'
    },

    /// Create multiple listings from extracted resource link
    CreatePostsFromResourceLink {
        job_id: JobId,
        url: String,
        posts: Vec<ExtractedPost>,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Update listing status (for approval/rejection)
    UpdatePostStatus {
        post_id: PostId,
        status: String,
        rejection_reason: Option<String>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Update listing content and approve it
    UpdatePostAndApprove {
        post_id: PostId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Create a post (when listing is approved)
    CreatePost {
        post_id: PostId,
        created_by: Option<MemberId>,
        custom_title: Option<String>,
        custom_description: Option<String>,
        expires_in_days: Option<i64>,
    },

    /// Generate embedding for a listing (background job)
    GeneratePostEmbedding { post_id: PostId },

    /// Create a custom post (admin-created post with custom content)
    CreateCustomPost {
        post_id: PostId,
        custom_title: Option<String>,
        custom_description: Option<String>,
        custom_tldr: Option<String>,
        targeting_hints: Option<JsonValue>,
        expires_in_days: Option<i64>,
        created_by: MemberId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Repost a listing (create new post for existing active listing)
    RepostPost {
        post_id: PostId,
        created_by: MemberId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Expire a post (mark as expired)
    ExpirePost {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Archive a post (mark as archived)
    ArchivePost {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Increment post view count (analytics)
    IncrementPostView { post_id: PostId },

    /// Increment post click count (analytics)
    IncrementPostClick { post_id: PostId },

    /// Delete a listing
    DeletePost {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Create a listing report
    CreateReport {
        post_id: PostId,
        reported_by: Option<MemberId>,
        reporter_email: Option<String>,
        reason: String,
        category: String,
    },

    /// Resolve a listing report
    ResolveReport {
        report_id: PostReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        action_taken: String,
        is_admin: bool,
    },

    /// Dismiss a listing report
    DismissReport {
        report_id: PostReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        is_admin: bool,
    },

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

    /// Extract listings from all crawled pages
    ExtractPostsFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
    },

    /// Retry website crawl after no listings found
    RetryWebsiteCrawl {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Mark website as having no listings (terminal state after max retries)
    MarkWebsiteNoPosts {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Sync listings extracted from crawled pages with database
    SyncCrawledPosts {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
        page_results: Vec<crate::domains::posts::events::PageExtractionResult>,
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

    /// Deduplicate posts using embedding similarity
    DeduplicatePosts {
        job_id: JobId,
        similarity_threshold: f32, // e.g., 0.95 for 95% similarity
        requested_by: MemberId,
        is_admin: bool,
    },
}

// Implement Command trait for seesaw-rs integration
impl seesaw_core::Command for PostCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        use seesaw_core::ExecutionMode;

        match self {
            // All commands run inline (no job worker implemented)
            Self::ScrapeSource { .. } => ExecutionMode::Inline,
            Self::ScrapeResourceLink { .. } => ExecutionMode::Inline,
            Self::ExtractPosts { .. } => ExecutionMode::Inline,
            Self::ExtractPostsFromResourceLink { .. } => ExecutionMode::Inline,
            Self::SyncPosts { .. } => ExecutionMode::Inline,
            Self::CreatePostEntry { .. } => ExecutionMode::Inline,
            Self::CreateWebsiteFromLink { .. } => ExecutionMode::Inline,
            Self::CreatePostsFromResourceLink { .. } => ExecutionMode::Inline,
            Self::UpdatePostStatus { .. } => ExecutionMode::Inline,
            Self::UpdatePostAndApprove { .. } => ExecutionMode::Inline,
            Self::CreatePost { .. } => ExecutionMode::Inline,
            Self::CreateCustomPost { .. } => ExecutionMode::Inline,
            Self::RepostPost { .. } => ExecutionMode::Inline,
            Self::ExpirePost { .. } => ExecutionMode::Inline,
            Self::ArchivePost { .. } => ExecutionMode::Inline,
            Self::IncrementPostView { .. } => ExecutionMode::Inline,
            Self::IncrementPostClick { .. } => ExecutionMode::Inline,
            Self::DeletePost { .. } => ExecutionMode::Inline,
            Self::CreateReport { .. } => ExecutionMode::Inline,
            Self::ResolveReport { .. } => ExecutionMode::Inline,
            Self::DismissReport { .. } => ExecutionMode::Inline,
            Self::GeneratePostEmbedding { .. } => ExecutionMode::Inline,
            // Crawling commands
            Self::CrawlWebsite { .. } => ExecutionMode::Inline,
            Self::ExtractPostsFromPages { .. } => ExecutionMode::Inline,
            Self::RetryWebsiteCrawl { .. } => ExecutionMode::Inline,
            Self::MarkWebsiteNoPosts { .. } => ExecutionMode::Inline,
            Self::SyncCrawledPosts { .. } => ExecutionMode::Inline,
            Self::RegeneratePosts { .. } => ExecutionMode::Inline,
            Self::RegeneratePageSummaries { .. } => ExecutionMode::Inline,
            Self::RegeneratePageSummary { .. } => ExecutionMode::Inline,
            Self::RegeneratePagePosts { .. } => ExecutionMode::Inline,
            Self::DeduplicatePosts { .. } => ExecutionMode::Inline,
        }
    }

    fn job_spec(&self) -> Option<seesaw_core::JobSpec> {
        match self {
            Self::ScrapeSource { source_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "scrape_source",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::ScrapeResourceLink { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "scrape_resource_link",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::ExtractPosts { source_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_posts",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::ExtractPostsFromResourceLink { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_posts_from_resource_link",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::SyncPosts { source_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "sync_posts",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::GeneratePostEmbedding { post_id } => Some(seesaw_core::JobSpec {
                job_type: "generate_post_embedding",
                idempotency_key: Some(post_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
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
            Self::RegeneratePageSummary {
                page_snapshot_id, ..
            } => Some(seesaw_core::JobSpec {
                job_type: "regenerate_page_summary",
                idempotency_key: Some(page_snapshot_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::RegeneratePagePosts {
                page_snapshot_id, ..
            } => Some(seesaw_core::JobSpec {
                job_type: "regenerate_page_posts",
                idempotency_key: Some(page_snapshot_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::DeduplicatePosts { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "deduplicate_posts",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 1,
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
