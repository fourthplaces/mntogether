// Common types used across multiple domains and layers
//
// These types are shared between the kernel and domain layers to avoid
// circular dependencies while maintaining type safety.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Import unified ContactInfo from extraction_types
use super::extraction_types::ContactInfo;

/// A schedule entry extracted from text by AI
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedSchedule {
    /// "weekly", "biweekly", "monthly", "one_time"
    pub frequency: String,
    /// Day of week: "monday", "tuesday", etc. For recurring schedules.
    #[serde(default)]
    pub day_of_week: Option<String>,
    /// Start time in "HH:MM" 24h format (e.g., "17:00")
    #[serde(default)]
    pub start_time: Option<String>,
    /// End time in "HH:MM" 24h format (e.g., "19:00")
    #[serde(default)]
    pub end_time: Option<String>,
    /// Specific date for one-off events: "2026-03-15"
    #[serde(default)]
    pub date: Option<String>,
    /// Freeform notes like "By appointment only", "1st and 3rd week only"
    #[serde(default)]
    pub notes: Option<String>,
}

/// A listing extracted from a website by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPost {
    pub title: String,
    pub summary: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    #[serde(default)]
    pub location: Option<String>,
    pub urgency: Option<String>,
    pub confidence: Option<String>, // "high" | "medium" | "low"
    /// The page snapshot this post was extracted from (for linking)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_page_snapshot_id: Option<Uuid>,
    /// Source URL(s) where this post was extracted from (may be comma-separated after dedup)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Structured location fields for proximity search
    #[serde(default)]
    pub zip_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    /// Dynamic tags from AI extraction, keyed by tag kind slug.
    /// e.g., {"post_type": ["service"], "population": ["refugees", "seniors"]}
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
    /// Extracted schedule entries (day/time/frequency)
    #[serde(default)]
    pub schedule: Vec<ExtractedSchedule>,
}

impl ExtractedPost {
    /// Combine a NarrativePost with investigation info into a complete ExtractedPost.
    ///
    /// Injects the narrative's audience as an `audience_role` tag entry,
    /// since Pass 1 sees the full page context when splitting by audience.
    pub fn from_narrative_and_info(
        narrative: crate::domains::crawling::activities::post_extraction::NarrativePost,
        info: ExtractedPostInformation,
    ) -> Self {
        let mut tags = TagEntry::to_map(&info.tags);

        // Inject narrative audience as audience_role tag
        let narrative_role = narrative.audience.to_lowercase();
        let audience_roles = tags.entry("audience_role".to_string()).or_default();
        if !audience_roles.contains(&narrative_role) {
            audience_roles.insert(0, narrative_role);
        }

        Self {
            title: narrative.title,
            summary: narrative.summary,
            description: narrative.description,
            contact: info.contact_or_none(),
            location: info.location,
            urgency: Some(info.urgency),
            confidence: Some(info.confidence),
            source_page_snapshot_id: None,
            source_url: Some(narrative.source_url),
            zip_code: info.zip_code,
            city: info.city,
            state: info.state,
            tags,
            schedule: info.schedule,
        }
    }
}

/// A listing extracted with its source URL (for batch extraction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPostWithSource {
    /// The source URL this listing was extracted from
    pub source_url: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    #[serde(default)]
    pub location: Option<String>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub schedule: Vec<ExtractedSchedule>,
}

impl ExtractedPostWithSource {
    /// Convert to ExtractedPost, preserving source_url
    pub fn into_post(self) -> ExtractedPost {
        ExtractedPost {
            title: self.title,
            summary: self.summary,
            description: self.description,
            contact: self.contact,
            location: self.location,
            urgency: self.urgency,
            confidence: self.confidence,
            source_page_snapshot_id: None,
            source_url: Some(self.source_url),
            zip_code: None,
            city: None,
            state: None,
            tags: self.tags,
            schedule: self.schedule,
        }
    }
}

/// A single tag classification entry for OpenAI structured output.
///
/// Uses a Vec of these instead of HashMap to be compatible with OpenAI strict mode
/// (which requires all object schemas to have named properties, not dynamic keys).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagEntry {
    /// Tag kind slug (e.g., "post_type", "population", "service_offered")
    pub kind: String,
    /// Tag values for this kind (e.g., ["service", "event"])
    pub values: Vec<String>,
}

impl TagEntry {
    /// Convert a Vec<TagEntry> to HashMap<String, Vec<String>> for downstream use.
    pub fn to_map(entries: &[TagEntry]) -> HashMap<String, Vec<String>> {
        entries
            .iter()
            .map(|e| (e.kind.clone(), e.values.clone()))
            .collect()
    }
}

/// Information extracted/investigated for a post.
///
/// This is the output of the agentic investigation step (Pass 2).
/// Combined with NarrativePost to create a complete ExtractedPost.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedPostInformation {
    pub contact: ContactInfo,
    pub location: Option<String>,
    pub urgency: String,
    pub confidence: String,
    #[serde(default)]
    pub zip_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    /// Tag classifications from AI extraction.
    /// Empty array when no tag kinds are configured.
    #[serde(default)]
    pub tags: Vec<TagEntry>,
    /// Extracted schedule entries (day/time/frequency)
    #[serde(default)]
    pub schedule: Vec<ExtractedSchedule>,
}

impl Default for ExtractedPostInformation {
    fn default() -> Self {
        Self {
            contact: ContactInfo::default(),
            location: None,
            urgency: "medium".to_string(),
            confidence: "low".to_string(),
            zip_code: None,
            city: None,
            state: None,
            tags: vec![TagEntry {
                kind: "audience_role".to_string(),
                values: vec!["recipient".to_string()],
            }],
            schedule: Vec::new(),
        }
    }
}

impl ExtractedPostInformation {
    /// Returns contact as Option, None if all fields are empty
    pub fn contact_or_none(&self) -> Option<ContactInfo> {
        if self.contact.phone.is_none()
            && self.contact.email.is_none()
            && self.contact.website.is_none()
            && self.contact.intake_form_url.is_none()
            && self.contact.contact_name.is_none()
        {
            None
        } else {
            Some(self.contact.clone())
        }
    }
}
