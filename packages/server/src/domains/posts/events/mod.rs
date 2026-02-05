// Import common types (shared across layers)
pub use crate::common::{ContactInfo, ExtractedPost};
use serde::{Deserialize, Serialize};

use crate::common::{JobId, PostId, WebsiteId};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostEvent {
    // =========================================================================
    // Fact Events (what actually happened)
    // =========================================================================
    /// User-submitted resource link was scraped successfully
    ResourceLinkScraped {
        job_id: JobId,
        url: String,
        content: String,
        context: Option<String>,
        submitter_contact: Option<String>,
        page_snapshot_id: Option<uuid::Uuid>, // Link to cached page content
    },

    /// AI extracted listings from user-submitted resource link
    ResourceLinkPostsExtracted {
        job_id: JobId,
        url: String,
        posts: Vec<ExtractedPost>,
        context: Option<String>,
        submitter_contact: Option<String>,
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
}

// NOTE: Failed/error events have been removed:
// - ScrapeFailed, ResourceLinkScrapeFailed, ExtractFailed, SyncFailed
// - ListingEmbeddingFailed, DeduplicationFailed
// - AuthorizationDenied
// Errors go in Result::Err, not in events. Events are for successful state changes.

// Note: All *Requested events have been removed.
// GraphQL mutations call actions directly via process().
// Effects watch FACT events and call cascade handlers directly.
