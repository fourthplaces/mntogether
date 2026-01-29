use serde_json::Value as JsonValue;
use std::net::IpAddr;

// Import common types (shared across layers)
pub use crate::common::{ContactInfo, ExtractedListing};
use crate::common::{JobId, ListingId, MemberId, PostId, SourceId};

/// Listings domain events
/// Following seesaw-rs pattern: Events are immutable facts
#[derive(Debug, Clone)]
pub enum ListingEvent {
    // =========================================================================
    // Request Events (from edges - entry points)
    // =========================================================================
    /// Admin requests to scrape an organization source
    ScrapeSourceRequested {
        source_id: SourceId,
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
    OrganizationSourceCreatedFromLink {
        source_id: SourceId,
        job_id: JobId,
        url: String,
        organization_name: String,
        submitter_contact: Option<String>,
    },

    /// Admin approves a listing (makes it active)
    ApproveListingRequested {
        listing_id: ListingId,
        requested_by: MemberId, // User making the request
        is_admin: bool,         // Whether user is admin (checked in effect)
    },

    /// Admin edits and approves a listing (fix AI mistakes)
    EditAndApproveListingRequested {
        listing_id: ListingId,
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
        listing_id: ListingId,
        reason: String,
        requested_by: MemberId, // User making the request
        is_admin: bool,         // Whether user is admin (checked in effect)
    },

    /// Admin creates a custom post for a listing
    CreateCustomPostRequested {
        listing_id: ListingId,
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
        listing_id: ListingId,
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
        listing_id: ListingId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin adds a scrape URL to an organization source
    AddScrapeUrlRequested {
        source_id: SourceId,
        url: String,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin removes a scrape URL from an organization source
    RemoveScrapeUrlRequested {
        source_id: SourceId,
        url: String,
        requested_by: MemberId,
        is_admin: bool,
    },

    // =========================================================================
    // Fact Events (from effects - what actually happened)
    // =========================================================================
    /// Source was scraped successfully
    SourceScraped {
        source_id: SourceId,
        job_id: JobId,
        organization_name: String,
        content: String,
    },

    /// User-submitted resource link was scraped successfully
    ResourceLinkScraped {
        job_id: JobId,
        url: String,
        content: String,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// AI extracted listings from scraped content
    ListingsExtracted {
        source_id: SourceId,
        job_id: JobId,
        listings: Vec<ExtractedListing>,
    },

    /// AI extracted listings from user-submitted resource link
    ResourceLinkListingsExtracted {
        job_id: JobId,
        url: String,
        listings: Vec<ExtractedListing>,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Listings were synced with database
    ListingsSynced {
        source_id: SourceId,
        job_id: JobId,
        new_count: usize,
        changed_count: usize,
        disappeared_count: usize,
    },

    /// Scraping failed (terminal event - clears pending state)
    ScrapeFailed {
        source_id: SourceId,
        job_id: JobId,
        reason: String,
    },

    /// Resource link scraping failed (terminal event)
    ResourceLinkScrapeFailed {
        job_id: JobId,
        reason: String,
    },

    /// Listing extraction failed (terminal event - clears pending state)
    ExtractFailed {
        source_id: SourceId,
        job_id: JobId,
        reason: String,
    },

    /// Listing sync failed (terminal event - clears pending state)
    SyncFailed {
        source_id: SourceId,
        job_id: JobId,
        reason: String,
    },

    /// A listing was created (from scraping or user submission)
    ListingCreated {
        listing_id: ListingId,
        organization_name: String,
        title: String,
        submission_type: String, // 'scraped' or 'user_submitted'
    },

    /// A listing was approved by admin
    ListingApproved { listing_id: ListingId },

    /// A listing was rejected by admin
    ListingRejected { listing_id: ListingId, reason: String },

    /// A listing was updated
    ListingUpdated { listing_id: ListingId },

    /// A post was created (when listing approved or custom post created)
    PostCreated { post_id: PostId, listing_id: ListingId },

    /// A post was expired
    PostExpired { post_id: PostId },

    /// A post was archived
    PostArchived { post_id: PostId },

    /// A post was viewed (analytics event)
    PostViewed { post_id: PostId },

    /// A post was clicked (analytics event)
    PostClicked { post_id: PostId },

    /// A listing was deleted
    ListingDeleted { listing_id: ListingId },

    /// A scrape URL was added to an organization source
    ScrapeUrlAdded {
        source_id: SourceId,
        url: String,
    },

    /// A scrape URL was removed from an organization source
    ScrapeUrlRemoved {
        source_id: SourceId,
        url: String,
    },

    /// Embedding generated for a listing
    ListingEmbeddingGenerated { listing_id: ListingId, dimensions: usize },

    /// Embedding generation failed for a listing
    ListingEmbeddingFailed { listing_id: ListingId, reason: String },

    // Authorization failures
    /// User attempted admin action without permission
    AuthorizationDenied {
        user_id: MemberId,
        action: String, // e.g., "ApproveListing", "ScrapeSource"
        reason: String,
    },

    // =========================================================================
    // Intelligent Crawler Events
    // =========================================================================
    /// Request to crawl a site intelligently
    SiteCrawlRequested {
        url: String,
        job_id: JobId,
    },

    /// Site was crawled successfully
    SiteCrawled {
        url: String,
        job_id: JobId,
        snapshot_ids: Vec<uuid::Uuid>,
    },

    /// Site crawl failed
    SiteCrawlFailed {
        url: String,
        job_id: JobId,
        reason: String,
    },

    /// Information was detected in crawled pages
    InformationDetected {
        job_id: JobId,
        detection_ids: Vec<uuid::Uuid>,
    },

    /// Information detection failed
    InformationDetectionFailed {
        job_id: JobId,
        reason: String,
    },

    /// Structured data was extracted from detections
    DataExtracted {
        job_id: JobId,
        extraction_ids: Vec<uuid::Uuid>,
    },

    /// Data extraction failed
    DataExtractionFailed {
        job_id: JobId,
        reason: String,
    },

    /// Relationships were resolved between extractions
    RelationshipsResolved {
        job_id: JobId,
        relationship_ids: Vec<uuid::Uuid>,
    },

    /// Relationship resolution failed
    RelationshipResolutionFailed {
        job_id: JobId,
        reason: String,
    },
}
