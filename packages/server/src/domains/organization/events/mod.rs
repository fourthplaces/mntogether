use serde_json::Value as JsonValue;
use std::net::IpAddr;
use uuid::Uuid;

/// Organization domain events
/// Following seesaw-rs pattern: Events are immutable facts
#[derive(Debug, Clone)]
pub enum OrganizationEvent {
    // =========================================================================
    // Request Events (from edges - entry points)
    // =========================================================================
    /// Admin requests to scrape an organization source
    ScrapeSourceRequested {
        source_id: Uuid,
        job_id: Uuid, // Track job for async workflow
    },

    /// Volunteer submits a need they encountered
    SubmitNeedRequested {
        volunteer_id: Option<Uuid>,
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
        need_id: Uuid,
    },

    /// Admin edits and approves a need (fix AI mistakes)
    EditAndApproveNeedRequested {
        need_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
    },

    /// Admin rejects a need (hide forever)
    RejectNeedRequested {
        need_id: Uuid,
        reason: String,
    },

    // =========================================================================
    // Fact Events (from effects - what actually happened)
    // =========================================================================
    /// Source was scraped successfully
    SourceScraped {
        source_id: Uuid,
        job_id: Uuid,
        organization_name: String,
        content: String,
    },

    /// AI extracted needs from scraped content
    NeedsExtracted {
        source_id: Uuid,
        job_id: Uuid,
        needs: Vec<ExtractedNeed>,
    },

    /// Needs were synced with database
    NeedsSynced {
        source_id: Uuid,
        job_id: Uuid,
        new_count: usize,
        changed_count: usize,
        disappeared_count: usize,
    },

    /// A need was created (from scraping or user submission)
    NeedCreated {
        need_id: Uuid,
        organization_name: String,
        title: String,
        submission_type: String, // 'scraped' or 'user_submitted'
    },

    /// A need was approved by admin
    NeedApproved {
        need_id: Uuid,
    },

    /// A need was rejected by admin
    NeedRejected {
        need_id: Uuid,
        reason: String,
    },

    /// A need was updated
    NeedUpdated {
        need_id: Uuid,
    },

    /// A post was created (when need approved)
    PostCreated {
        post_id: Uuid,
        need_id: Uuid,
    },

    /// Embedding generated for a need
    NeedEmbeddingGenerated {
        need_id: Uuid,
        dimensions: usize,
    },

    /// Embedding generation failed for a need
    NeedEmbeddingFailed {
        need_id: Uuid,
        reason: String,
    },
}

/// Extracted need from AI
#[derive(Debug, Clone)]
pub struct ExtractedNeed {
    pub title: String,
    pub description: String,
    pub tldr: String,
    pub contact: Option<ContactInfo>,
    pub urgency: Option<String>,
    pub confidence: Option<String>, // "high" | "medium" | "low"
}

#[derive(Debug, Clone)]
pub struct ContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}
