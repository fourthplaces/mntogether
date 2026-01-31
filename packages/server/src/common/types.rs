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

/// Contact information for a need/opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}
