// Import common types (shared across layers)
pub use crate::common::{ContactInfo, ExtractedPost};
use crate::common::{JobId, MemberId, PostId, WebsiteId};
use crate::domains::posts::models::post_report::PostReportId;

/// Posts domain events
///
/// Architecture (seesaw 0.6.0 direct-call pattern):
///   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
///
/// NO *Requested events - GraphQL calls actions directly via process().
/// Effects watch FACT events and call cascade handlers directly.
///
/// NOTE: Crawling events have been moved to the `crawling` domain.
/// See `crate::domains::crawling::events::CrawlEvent`.
#[derive(Debug, Clone)]
pub enum PostEvent {
    // =========================================================================
    // Fact Events (what actually happened)
    // =========================================================================
    /// Source was scraped successfully
    SourceScraped {
        source_id: WebsiteId,
        job_id: JobId,
        organization_name: String,
        content: String,
        page_snapshot_id: Option<uuid::Uuid>, // Link to cached page content
    },

    /// A single page snapshot was refreshed (re-scraped)
    PageSnapshotRefreshed {
        page_snapshot_id: uuid::Uuid,
        job_id: JobId,
        url: String,
        content: String,
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
    PostRejected { post_id: PostId, reason: String },

    /// A listing was updated
    ListingUpdated { post_id: PostId },

    /// A post was created (when listing approved or custom post created)
    PostCreated { post_id: PostId },

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
    PostEmbeddingGenerated { post_id: PostId, dimensions: usize },

    /// Embedding generation failed for a listing
    ListingEmbeddingFailed { post_id: PostId, reason: String },

    // Authorization failures
    /// User attempted admin action without permission
    AuthorizationDenied {
        user_id: MemberId,
        action: String, // e.g., "ApproveListing", "ScrapeSource"
        reason: String,
    },

    // =========================================================================
    // Transition Events (workflow state changes, not entry points)
    // =========================================================================
    /// Organization source created from user-submitted link
    /// Triggers: handle_scrape_resource_link cascade
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

    // =========================================================================
    // Deduplication Events
    // =========================================================================
    /// Posts deduplicated successfully
    PostsDeduplicated {
        job_id: JobId,
        duplicates_found: usize,
        posts_merged: usize,
        posts_deleted: usize,
    },

    /// Deduplication failed
    DeduplicationFailed { job_id: JobId, reason: String },
}

// Note: All *Requested events have been removed.
// GraphQL mutations call actions directly via process().
// Effects watch FACT events and call cascade handlers directly.
