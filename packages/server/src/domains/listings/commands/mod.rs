use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::net::IpAddr;

use crate::common::{JobId, ListingId, MemberId, PostId, DomainId};
use crate::domains::listings::events::ExtractedListing;

/// Listings domain commands
/// Following seesaw-rs pattern: Commands are requests for IO operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListingCommand {
    /// Scrape a source URL using Firecrawl
    ScrapeSource {
        source_id: DomainId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Create organization source from user-submitted link
    CreateDomainFromLink {
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
    ExtractListings {
        source_id: DomainId,
        job_id: JobId,
        organization_name: String,
        content: String,
    },

    /// Extract listings from user-submitted resource link
    ExtractListingsFromResourceLink {
        job_id: JobId,
        url: String,
        content: String,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Sync extracted listings with database
    SyncListings {
        source_id: DomainId,
        job_id: JobId,
        listings: Vec<ExtractedListing>,
    },

    /// Create a new listing (from user submission)
    CreateListing {
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
    CreateListingsFromResourceLink {
        job_id: JobId,
        url: String,
        listings: Vec<ExtractedListing>,
        context: Option<String>,
        submitter_contact: Option<String>,
    },

    /// Update listing status (for approval/rejection)
    UpdateListingStatus {
        listing_id: ListingId,
        status: String,
        rejection_reason: Option<String>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Update listing content and approve it
    UpdateListingAndApprove {
        listing_id: ListingId,
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
        listing_id: ListingId,
        created_by: Option<MemberId>,
        custom_title: Option<String>,
        custom_description: Option<String>,
        expires_in_days: Option<i64>,
    },

    /// Generate embedding for a listing (background job)
    GenerateListingEmbedding { listing_id: ListingId },

    /// Create a custom post (admin-created post with custom content)
    CreateCustomPost {
        listing_id: ListingId,
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
    RepostListing {
        listing_id: ListingId,
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
    DeleteListing {
        listing_id: ListingId,
        requested_by: MemberId,
        is_admin: bool,
    },

    // Intelligent Crawler Commands (future use)
    /// Crawl a site using intelligent crawler
    CrawlSite {
        url: String,
        job_id: JobId,
        page_limit: Option<usize>,
    },

    /// Detect information in crawled pages
    DetectInformation {
        snapshot_ids: Vec<uuid::Uuid>,
        job_id: JobId,
        detection_kind: String,
    },

    /// Extract structured data from detections
    ExtractData {
        detection_ids: Vec<uuid::Uuid>,
        job_id: JobId,
        schema_id: uuid::Uuid,
    },

    /// Resolve relationships between extractions
    ResolveRelationships {
        extraction_ids: Vec<uuid::Uuid>,
        job_id: JobId,
    },

    /// Execute search for an agent
    ExecuteSearch {
        agent_id: uuid::Uuid,
        job_id: JobId,
    },
}

// Implement Command trait for seesaw-rs integration
impl seesaw_core::Command for ListingCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        use seesaw_core::ExecutionMode;

        match self {
            // All commands run inline (no job worker implemented)
            Self::ScrapeSource { .. } => ExecutionMode::Inline,
            Self::ScrapeResourceLink { .. } => ExecutionMode::Inline,
            Self::ExtractListings { .. } => ExecutionMode::Inline,
            Self::ExtractListingsFromResourceLink { .. } => ExecutionMode::Inline,
            Self::SyncListings { .. } => ExecutionMode::Inline,
            Self::CreateListing { .. } => ExecutionMode::Inline,
            Self::CreateDomainFromLink { .. } => ExecutionMode::Inline,
            Self::CreateListingsFromResourceLink { .. } => ExecutionMode::Inline,
            Self::UpdateListingStatus { .. } => ExecutionMode::Inline,
            Self::UpdateListingAndApprove { .. } => ExecutionMode::Inline,
            Self::CreatePost { .. } => ExecutionMode::Inline,
            Self::CreateCustomPost { .. } => ExecutionMode::Inline,
            Self::RepostListing { .. } => ExecutionMode::Inline,
            Self::ExpirePost { .. } => ExecutionMode::Inline,
            Self::ArchivePost { .. } => ExecutionMode::Inline,
            Self::IncrementPostView { .. } => ExecutionMode::Inline,
            Self::IncrementPostClick { .. } => ExecutionMode::Inline,
            Self::DeleteListing { .. } => ExecutionMode::Inline,
            Self::GenerateListingEmbedding { .. } => ExecutionMode::Inline,
            Self::CrawlSite { .. } => ExecutionMode::Inline,
            Self::DetectInformation { .. } => ExecutionMode::Inline,
            Self::ExtractData { .. } => ExecutionMode::Inline,
            Self::ResolveRelationships { .. } => ExecutionMode::Inline,
            Self::ExecuteSearch { .. } => ExecutionMode::Inline,
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
            Self::ExtractListings { source_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_listings",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::ExtractListingsFromResourceLink { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_listings_from_resource_link",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::SyncListings { source_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "sync_listings",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::GenerateListingEmbedding { listing_id } => Some(seesaw_core::JobSpec {
                job_type: "generate_listing_embedding",
                idempotency_key: Some(listing_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::CrawlSite { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "crawl_site",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
            }),
            Self::DetectInformation { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "detect_information",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::ExtractData { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_data",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::ResolveRelationships { job_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "resolve_relationships",
                idempotency_key: Some(job_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
            Self::ExecuteSearch { agent_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "execute_search",
                idempotency_key: Some(agent_id.to_string()),
                max_retries: 3,
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
