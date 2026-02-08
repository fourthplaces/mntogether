use crate::domains::posts::models::Post;
use serde::{Deserialize, Serialize};

/// API representation of a listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tldr: Option<String>,

    // Hot path fields
    pub post_type: String,
    pub category: String,
    pub capacity_status: Option<String>,
    pub urgency: Option<String>,
    pub status: String,

    // Verification
    pub verified_at: Option<String>,

    // Language
    pub source_language: String,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Timestamps
    pub created_at: String,
    pub updated_at: String,

    // Source tracking
    pub source_url: Option<String>,
}

/// Service-specific properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePostData {
    // Access & Requirements
    pub requires_identification: bool,
    pub requires_appointment: bool,
    pub walk_ins_accepted: bool,

    // Delivery Methods
    pub remote_available: bool,
    pub in_person_available: bool,
    pub home_visits_available: bool,

    // Accessibility
    pub wheelchair_accessible: bool,
    pub interpretation_available: bool,

    // Costs
    pub free_service: bool,
    pub sliding_scale_fees: bool,
    pub accepts_insurance: bool,

    // Hours
    pub evening_hours: bool,
    pub weekend_hours: bool,
}

impl From<Post> for PostData {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.to_string(),
            title: post.title,
            description: post.description,
            tldr: post.tldr,
            post_type: post.post_type,
            category: post.category,
            capacity_status: post.capacity_status,
            urgency: post.urgency,
            status: post.status,
            verified_at: post.verified_at.map(|dt| dt.to_rfc3339()),
            source_language: post.source_language,
            location: post.location,
            latitude: post.latitude,
            longitude: post.longitude,
            created_at: post.created_at.to_rfc3339(),
            updated_at: post.updated_at.to_rfc3339(),
            source_url: post.source_url,
        }
    }
}
