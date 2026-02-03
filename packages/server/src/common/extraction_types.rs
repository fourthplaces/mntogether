//! Unified extraction types used across all domains.
//!
//! This is the SINGLE source of truth for extraction-related types.
//! All domains should import from here instead of defining their own versions.

use serde::{Deserialize, Serialize};

/// Contact information - unified across all extraction contexts.
///
/// Superset of all fields needed by crawling, posts, and organization domains.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub intake_form_url: Option<String>,
    pub contact_name: Option<String>,
    /// Additional contact methods (fax, TTY, etc.)
    #[serde(default)]
    pub other: Vec<String>,
}

/// Location/address information - unified across all extraction contexts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocationInfo {
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    /// Service area description (e.g., "Twin Cities metro", "Statewide")
    pub service_area: Option<String>,
    /// Whether this is a virtual/remote service
    #[serde(default)]
    pub is_virtual: bool,
}

/// Schedule/hours information - unified across all extraction contexts.
///
/// Combines fields from HoursInfo (crawling) and ScheduleInfo (posts).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScheduleInfo {
    /// General hours description (e.g., "Mon-Fri 9am-5pm")
    pub general: Option<String>,
    /// Specific dates if applicable
    pub dates: Option<String>,
    /// Frequency (e.g., "weekly", "monthly")
    pub frequency: Option<String>,
    /// Duration (e.g., "2 hours")
    pub duration: Option<String>,
    /// Structured hours by day if available
    #[serde(default)]
    pub by_day: Vec<DayHours>,
    /// Notes about hours (holidays, seasonal changes, etc.)
    pub notes: Option<String>,
}

/// Hours for a specific day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayHours {
    pub day: String,
    pub hours: String,
}

/// Eligibility/requirements information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EligibilityInfo {
    pub who_qualifies: Option<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    pub restrictions: Option<String>,
}

/// Call to action - how to engage with a post/service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToAction {
    pub action: String,
    pub url: Option<String>,
    pub instructions: Option<String>,
}

/// Extraction type enum - replaces string constants.
///
/// Use this instead of string literals like "summary", "posts", etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractionType {
    Summary,
    Posts,
    Contacts,
    Hours,
    Events,
}

impl ExtractionType {
    /// Get the string representation (for database storage).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Posts => "posts",
            Self::Contacts => "contacts",
            Self::Hours => "hours",
            Self::Events => "events",
        }
    }
}

impl std::fmt::Display for ExtractionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ExtractionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "summary" => Ok(Self::Summary),
            "posts" => Ok(Self::Posts),
            "contacts" => Ok(Self::Contacts),
            "hours" => Ok(Self::Hours),
            "events" => Ok(Self::Events),
            _ => Err(format!("Unknown extraction type: {}", s)),
        }
    }
}
