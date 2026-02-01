use serde_json::Value as JsonValue;

// Import common types (shared across layers)
pub use crate::common::{ContactInfo, ExtractedPost};
use crate::common::{JobId, MemberId, PostId, WebsiteId};
use crate::domains::posts::models::post_report::PostReportId;

/// Posts domain events
/// Following seesaw-rs pattern: Events are immutable facts
///
/// NOTE: Crawling events have been moved to the `crawling` domain.
/// See `crate::domains::crawling::events::CrawlEvent`.
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
    RepostPostRequested {
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
    DeletePostRequested {
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
    GeneratePostEmbeddingRequested { post_id: PostId },

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
    PostsExtracted {
        source_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,
    },

    /// AI extracted listings from user-submitted resource link
    ResourceLinkPostsExtracted {
        job_id: JobId,
        url: String,
        posts: Vec<ExtractedPost>,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Listings were synced with database
    PostsSynced {
        source_id: WebsiteId,
        job_id: JobId,
        new_count: usize,
        updated_count: usize,
        unchanged_count: usize,
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
    PostEntryCreated {
        post_id: PostId,
        organization_name: String,
        title: String,
        submission_type: String, // 'scraped' or 'user_submitted'
    },

    /// A listing was approved by admin
    PostApproved { post_id: PostId },

    /// A listing was rejected by admin
    PostRejected {
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
    PostDeleted { post_id: PostId },

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
    PostEmbeddingGenerated {
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
    // Deduplication Events
    // =========================================================================
    /// Admin requests to deduplicate posts using embedding similarity
    DeduplicatePostsRequested {
        job_id: JobId,
        similarity_threshold: f32,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Posts deduplicated successfully
    PostsDeduplicated {
        job_id: JobId,
        duplicates_found: usize,
        posts_merged: usize,
        posts_deleted: usize,
    },

    /// Deduplication failed
    DeduplicationFailed {
        job_id: JobId,
        reason: String,
    },
}
