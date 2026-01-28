use serde_json::Value as JsonValue;
use std::net::IpAddr;

// Import common types (shared across layers)
pub use crate::common::{ContactInfo, ExtractedNeed};
use crate::common::{JobId, MemberId, NeedId, PostId, SourceId};

/// Organization domain events
/// Following seesaw-rs pattern: Events are immutable facts
#[derive(Debug, Clone)]
pub enum OrganizationEvent {
    // =========================================================================
    // Request Events (from edges - entry points)
    // =========================================================================
    /// Admin requests to scrape an organization source
    ScrapeSourceRequested {
        source_id: SourceId,
        job_id: JobId, // Track job for async workflow
        requested_by: MemberId, // User making the request (for authorization)
        is_admin: bool, // Whether user is admin (checked in effect)
    },

    /// Member submits a need they encountered
    SubmitNeedRequested {
        member_id: Option<MemberId>,
        organization_name: String,
        title: String,
        description: String,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        ip_address: Option<String>,
    },

    /// Admin approves a need (makes it active)
    ApproveNeedRequested {
        need_id: NeedId,
        requested_by: MemberId, // User making the request
        is_admin: bool, // Whether user is admin (checked in effect)
    },

    /// Admin edits and approves a need (fix AI mistakes)
    EditAndApproveNeedRequested {
        need_id: NeedId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        requested_by: MemberId, // User making the request
        is_admin: bool, // Whether user is admin (checked in effect)
    },

    /// Admin rejects a need (hide forever)
    RejectNeedRequested {
        need_id: NeedId,
        reason: String,
        requested_by: MemberId, // User making the request
        is_admin: bool, // Whether user is admin (checked in effect)
    },

    /// Admin creates a custom post for a need
    CreateCustomPostRequested {
        need_id: NeedId,
        custom_title: Option<String>,
        custom_description: Option<String>,
        custom_tldr: Option<String>,
        targeting_hints: Option<JsonValue>,
        expires_in_days: Option<i64>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Admin reposts a need (creates new post for existing need)
    RepostNeedRequested {
        need_id: NeedId,
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
    PostViewedRequested {
        post_id: PostId,
    },

    /// Member clicked on a post (analytics)
    PostClickedRequested {
        post_id: PostId,
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

    /// AI extracted needs from scraped content
    NeedsExtracted {
        source_id: SourceId,
        job_id: JobId,
        needs: Vec<ExtractedNeed>,
    },

    /// Needs were synced with database
    NeedsSynced {
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

    /// Need extraction failed (terminal event - clears pending state)
    ExtractFailed {
        source_id: SourceId,
        job_id: JobId,
        reason: String,
    },

    /// Need sync failed (terminal event - clears pending state)
    SyncFailed {
        source_id: SourceId,
        job_id: JobId,
        reason: String,
    },

    /// A need was created (from scraping or user submission)
    NeedCreated {
        need_id: NeedId,
        organization_name: String,
        title: String,
        submission_type: String, // 'scraped' or 'user_submitted'
    },

    /// A need was approved by admin
    NeedApproved { need_id: NeedId },

    /// A need was rejected by admin
    NeedRejected { need_id: NeedId, reason: String },

    /// A need was updated
    NeedUpdated { need_id: NeedId },

    /// A post was created (when need approved or custom post created)
    PostCreated { post_id: PostId, need_id: NeedId },

    /// A post was expired
    PostExpired { post_id: PostId },

    /// A post was archived
    PostArchived { post_id: PostId },

    /// A post was viewed (analytics event)
    PostViewed { post_id: PostId },

    /// A post was clicked (analytics event)
    PostClicked { post_id: PostId },

    /// Embedding generated for a need
    NeedEmbeddingGenerated { need_id: NeedId, dimensions: usize },

    /// Embedding generation failed for a need
    NeedEmbeddingFailed { need_id: NeedId, reason: String },

    // Authorization failures
    /// User attempted admin action without permission
    AuthorizationDenied {
        user_id: MemberId,
        action: String, // e.g., "ApproveNeed", "ScrapeSource"
        reason: String,
    },
}
