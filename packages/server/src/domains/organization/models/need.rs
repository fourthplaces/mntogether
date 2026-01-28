use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Organization need - a volunteer opportunity extracted from a website or submitted by user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationNeed {
    pub id: Uuid,
    pub organization_name: String,

    // Content
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,

    // Contact
    pub contact_info: Option<JsonValue>,

    // Metadata
    pub urgency: Option<String>,
    pub status: NeedStatus,
    pub content_hash: Option<String>,
    pub location: Option<String>,

    // Submission tracking
    pub submission_type: Option<String>, // 'scraped' | 'user_submitted'
    pub submitted_by_volunteer_id: Option<Uuid>,

    // Sync tracking (for scraped needs)
    pub source_id: Option<Uuid>,
    pub last_seen_at: DateTime<Utc>,
    pub disappeared_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NeedStatus {
    PendingApproval, // AI extracted, awaiting human review
    Active,          // Approved by admin, visible to volunteers
    Rejected,        // Rejected by admin, hidden forever
    Expired,         // Auto-expired (no longer seen on website)
}

impl std::fmt::Display for NeedStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NeedStatus::PendingApproval => write!(f, "pending_approval"),
            NeedStatus::Active => write!(f, "active"),
            NeedStatus::Rejected => write!(f, "rejected"),
            NeedStatus::Expired => write!(f, "expired"),
        }
    }
}
