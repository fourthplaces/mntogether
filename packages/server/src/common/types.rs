// Common types used across multiple domains and layers
//
// These types are shared between the kernel and domain layers to avoid
// circular dependencies while maintaining type safety.

use serde::{Deserialize, Serialize};

/// A listing extracted from a website by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedListing {
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    pub urgency: Option<String>,
    pub confidence: Option<String>, // "high" | "medium" | "low"
    /// Target audience roles: who should engage with this listing
    /// Values: "recipient", "donor", "volunteer", "participant"
    #[serde(default)]
    pub audience_roles: Vec<String>,
}

/// A listing extracted with its source URL (for batch extraction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedListingWithSource {
    /// The source URL this listing was extracted from
    pub source_url: String,
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    #[serde(default)]
    pub audience_roles: Vec<String>,
}

impl ExtractedListingWithSource {
    /// Convert to ExtractedListing (dropping source_url)
    pub fn into_listing(self) -> ExtractedListing {
        ExtractedListing {
            title: self.title,
            tldr: self.tldr,
            description: self.description,
            contact: self.contact,
            urgency: self.urgency,
            confidence: self.confidence,
            audience_roles: self.audience_roles,
        }
    }
}

/// Contact information for a need/opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}
