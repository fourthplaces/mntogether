// Common types used across multiple domains and layers
//
// These types are shared between the kernel and domain layers to avoid
// circular dependencies while maintaining type safety.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Import unified ContactInfo from extraction_types
use super::extraction_types::ContactInfo;

/// A listing extracted from a website by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPost {
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    #[serde(default)]
    pub location: Option<String>,
    pub urgency: Option<String>,
    pub confidence: Option<String>, // "high" | "medium" | "low"
    /// Target audience roles: who should engage with this listing
    /// Values: "recipient", "donor", "volunteer", "participant"
    #[serde(default)]
    pub audience_roles: Vec<String>,
    /// The page snapshot this post was extracted from (for linking)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_page_snapshot_id: Option<Uuid>,
}

/// A listing extracted with its source URL (for batch extraction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPostWithSource {
    /// The source URL this listing was extracted from
    pub source_url: String,
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    #[serde(default)]
    pub location: Option<String>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    #[serde(default)]
    pub audience_roles: Vec<String>,
}

impl ExtractedPostWithSource {
    /// Convert to ExtractedPost (dropping source_url)
    pub fn into_post(self) -> ExtractedPost {
        ExtractedPost {
            title: self.title,
            tldr: self.tldr,
            description: self.description,
            contact: self.contact,
            location: self.location,
            urgency: self.urgency,
            confidence: self.confidence,
            audience_roles: self.audience_roles,
            source_page_snapshot_id: None, // ExtractedPostWithSource doesn't have this
        }
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
    pub audience_roles: Vec<String>,
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
