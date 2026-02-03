//! Types for page summarization
//!
//! Used for AI-generated page summaries displayed in admin UI.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Import unified types from common
use crate::common::{ContactInfo, LocationInfo, ScheduleInfo};

/// Input for Pass 1: a page to summarize
#[derive(Debug, Clone)]
pub struct PageToSummarize {
    pub snapshot_id: Uuid,
    pub url: String,
    pub raw_content: String,  // Raw HTML/markdown from snapshot
    pub content_hash: String, // For cache lookup
}

/// Output from Pass 1: structured content extracted from a page
#[derive(Debug, Clone)]
pub struct SummarizedPage {
    pub snapshot_id: Uuid,
    pub url: String,
    pub content: String, // JSON string of PageSummaryContent
}

/// Structured content extracted from a single page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageSummaryContent {
    /// Organization info found on this page
    #[serde(default)]
    pub organization: Option<OrganizationInfo>,
    /// Distinct programs/services found on this page
    #[serde(default)]
    pub programs: Vec<ProgramInfo>,
    /// Contact information found on this page
    #[serde(default)]
    pub contact: Option<ContactInfo>,
    /// Location/address information
    #[serde(default)]
    pub location: Option<LocationInfo>,
    /// Hours of operation (uses unified ScheduleInfo)
    #[serde(default)]
    pub hours: Option<ScheduleInfo>,
    /// Events or time-sensitive items
    #[serde(default)]
    pub events: Vec<EventInfo>,
    /// Raw text summary for context that doesn't fit structured fields
    #[serde(default)]
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    pub name: Option<String>,
    pub mission: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub languages_served: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    pub name: String,
    /// Plain English description - what this program offers, written for someone seeking help.
    /// This should be readable and user-friendly, not just keywords.
    pub description: Option<String>,
    /// Who this program serves (e.g., "families with children", "seniors 60+")
    pub serves: Option<String>,
    /// How to access this program (e.g., "Walk in during open hours", "Call to make appointment")
    pub how_to_access: Option<String>,
    /// What to bring or eligibility requirements
    pub eligibility: Option<String>,
    /// Program-specific contact if different from org
    pub contact: Option<ContactInfo>,
    /// Program-specific hours if different from org
    pub hours: Option<String>,
    /// Program-specific location if different from org
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    pub name: String,
    pub date: Option<String>,
    pub time: Option<String>,
    pub description: Option<String>,
    pub registration_info: Option<String>,
}

/// Type alias for backwards compatibility - HoursInfo is now ScheduleInfo
pub type HoursInfo = ScheduleInfo;
