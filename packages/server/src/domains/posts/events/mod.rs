use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// Import common types (shared across layers)
pub use crate::common::{ContactInfo, ExtractedPost};
use crate::common::{JobId, PostId, MemberId, WebsiteId};
use crate::domains::posts::models::post_report::PostReportId;

/// Information about a crawled page (used in WebsiteCrawled event)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPageInfo {
    pub url: String,
    pub title: Option<String>,
    pub snapshot_id: Option<uuid::Uuid>,
}

/// Result of extracting listings from a single page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageExtractionResult {
    pub url: String,
    pub snapshot_id: Option<uuid::Uuid>,
    pub listings_count: usize,
    pub has_listings: bool,
}

/// Listings domain events
/// Following seesaw-rs pattern: Events are immutable facts
#[derive(Debug, Clone)]
pub enum PostEvent {
    // =========================================================================
    // Request Events (from edges - entry points)
    // =========================================================================
    /// Admin requests to scrape an organization source
    ScrapeSourceRequested {
        source_id: WebsiteId,
        job_id: JobId,          // Track job for async workflow
        requested_by: MemberId, // User making the request (for authorization)
        is_admin: bool,         // Whether user is admin (checked in effect)
    },

    /// Admin requests to crawl a website (multi-page)
    CrawlWebsiteRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Member submits a listing they encountered
    SubmitListingRequested {
        member_id: Option<MemberId>,
        organization_name: String,
        title: String,
        description: String,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        ip_address: Option<String>,
    },

    /// Public user submits a resource link (URL) for scraping
    SubmitResourceLinkRequested {
        url: String,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Organization source created from user-submitted link
    WebsiteCreatedFromLink {
        source_id: WebsiteId,
        job_id: JobId,
        url: String,
        organization_name: String,
        submitter_contact: Option<String>,
    },

    /// Website created but pending admin approval before scraping
    WebsitePendingApproval {
        website_id: WebsiteId,
        url: String,
        submitted_url: String,
        submitter_contact: Option<String>,
    },

    /// Admin approves a listing (makes it active)
    ApproveListingRequested {
        post_id: PostId,
        requested_by: MemberId, // User making the request
        is_admin: bool,         // Whether user is admin (checked in effect)
    },

    /// Admin edits and approves a listing (fix AI mistakes)
    EditAndApproveListingRequested {
        post_id: PostId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        requested_by: MemberId, // User making the request
        is_admin: bool,         // Whether user is admin (checked in effect)
    },

    /// Admin rejects a listing (hide forever)
    RejectListingRequested {
        post_id: PostId,
        reason: String,
        requested_by: MemberId, // User making the request
        is_admin: bool,         // Whether user is admin (checked in effect)
    },

    /// Admin creates a custom post for a listing
    CreateCustomPostRequested {
        post_id: PostId,
        custom_title: Option<String>,
        custom_description: Option<String>,
        custom_tldr: Option<String>,
        targeting_hints: Option<JsonValue>,
        expires_in_days: Option<i64>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin reposts a listing (creates new post for existing listing)
    RepostListingRequested {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin expires a post
    ExpirePostRequested {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin archives a post
    ArchivePostRequested {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Member viewed a post (analytics)
    PostViewedRequested { post_id: PostId },

    /// Member clicked on a post (analytics)
    PostClickedRequested { post_id: PostId },

    /// Admin deletes a listing
    DeleteListingRequested {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// User reports a listing for moderation
    ReportListingRequested {
        post_id: PostId,
        reported_by: Option<MemberId>,
        reporter_email: Option<String>,
        reason: String,
        category: String,
    },

    /// Admin resolves a report
    ResolveReportRequested {
        report_id: PostReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        action_taken: String,
        is_admin: bool,
    },

    /// Admin dismisses a report
    DismissReportRequested {
        report_id: PostReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        is_admin: bool,
    },

    /// Request to generate embedding for a single post
    GenerateListingEmbeddingRequested { post_id: PostId },

    // =========================================================================
    // Fact Events (from effects - what actually happened)
    // =========================================================================
    /// Source was scraped successfully
    SourceScraped {
        source_id: WebsiteId,
        job_id: JobId,
        organization_name: String,
        content: String,
        page_snapshot_id: Option<uuid::Uuid>, // Link to cached page content
    },

    /// User-submitted resource link was scraped successfully
    ResourceLinkScraped {
        job_id: JobId,
        url: String,
        content: String,
        context: Option<String>,
        submitter_contact: Option<String>,
        page_snapshot_id: Option<uuid::Uuid>, // Link to cached page content
    },

    /// AI extracted listings from scraped content
    ListingsExtracted {
        source_id: WebsiteId,
        job_id: JobId,
        listings: Vec<ExtractedPost>,
    },

    /// AI extracted listings from user-submitted resource link
    ResourceLinkListingsExtracted {
        job_id: JobId,
        url: String,
        listings: Vec<ExtractedPost>,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Listings were synced with database
    ListingsSynced {
        source_id: WebsiteId,
        job_id: JobId,
        new_count: usize,
        changed_count: usize,
        disappeared_count: usize,
    },

    /// Scraping failed (terminal event - clears pending state)
    ScrapeFailed {
        source_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    /// Resource link scraping failed (terminal event)
    ResourceLinkScrapeFailed { job_id: JobId, reason: String },

    /// Listing extraction failed (terminal event - clears pending state)
    ExtractFailed {
        source_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    /// Listing sync failed (terminal event - clears pending state)
    SyncFailed {
        source_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    /// A listing was created (from scraping or user submission)
    ListingCreated {
        post_id: PostId,
        organization_name: String,
        title: String,
        submission_type: String, // 'scraped' or 'user_submitted'
    },

    /// A listing was approved by admin
    ListingApproved { post_id: PostId },

    /// A listing was rejected by admin
    ListingRejected {
        post_id: PostId,
        reason: String,
    },

    /// A listing was updated
    ListingUpdated { post_id: PostId },

    /// A post was created (when listing approved or custom post created)
    PostCreated {
        post_id: PostId,
    },

    /// A post was expired
    PostExpired { post_id: PostId },

    /// A post was archived
    PostArchived { post_id: PostId },

    /// A post was viewed (analytics event)
    PostViewed { post_id: PostId },

    /// A post was clicked (analytics event)
    PostClicked { post_id: PostId },

    /// A listing was deleted
    ListingDeleted { post_id: PostId },

    /// A listing was reported
    PostReported {
        report_id: PostReportId,
        post_id: PostId,
    },

    /// A report was resolved
    ReportResolved {
        report_id: PostReportId,
        action_taken: String,
    },

    /// A report was dismissed
    ReportDismissed { report_id: PostReportId },

    /// Embedding generated for a listing
    ListingEmbeddingGenerated {
        post_id: PostId,
        dimensions: usize,
    },

    /// Embedding generation failed for a listing
    ListingEmbeddingFailed {
        post_id: PostId,
        reason: String,
    },

    // Authorization failures
    /// User attempted admin action without permission
    AuthorizationDenied {
        user_id: MemberId,
        action: String, // e.g., "ApproveListing", "ScrapeSource"
        reason: String,
    },

    // =========================================================================
    // Website Crawl Events (multi-page crawling workflow)
    // =========================================================================
    /// Website was crawled (multiple pages discovered)
    WebsiteCrawled {
        website_id: WebsiteId,
        job_id: JobId,
        pages: Vec<CrawledPageInfo>,
    },

    /// No listings found after crawling all pages
    WebsiteCrawlNoListings {
        website_id: WebsiteId,
        job_id: JobId,
        attempt_number: i32,
        pages_crawled: usize,
        should_retry: bool,
    },

    /// Terminal: website marked as having no listings after max retries
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

    /// Listings extracted from multiple crawled pages
    ListingsExtractedFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        listings: Vec<ExtractedPost>,
        page_results: Vec<PageExtractionResult>,
    },

    /// Admin requests to regenerate posts from existing page snapshots
    /// (skips crawling, goes directly to extraction)
    RegeneratePostsRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin requests to regenerate page summaries for existing snapshots
    /// (re-runs AI summarization, clears cached summaries)
    RegeneratePageSummariesRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Page summaries regenerated successfully
    PageSummariesRegenerated {
        website_id: WebsiteId,
        job_id: JobId,
        pages_processed: usize,
    },

    /// Admin requests to regenerate AI summary for a single page snapshot
    RegeneratePageSummaryRequested {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Single page summary regenerated successfully
    PageSummaryRegenerated {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
    },

    /// Admin requests to regenerate posts for a single page snapshot
    RegeneratePagePostsRequested {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Single page posts regenerated successfully
    PagePostsRegenerated {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        posts_count: usize,
    },
}
