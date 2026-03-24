use crate::domains::posts::models::Post;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

/// API representation of a listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostData {
    pub id: String,
    pub title: String,
    pub body_raw: String,

    // Hot path fields
    pub post_type: String,
    pub status: String,
    pub is_urgent: bool,

    // Language
    pub source_language: String,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Timestamps
    pub created_at: String,
    pub updated_at: String,

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
            body_raw: post.body_raw,
            post_type: post.post_type,
            status: post.status,
            is_urgent: post.is_urgent,
            source_language: post.source_language,
            location: post.location,
            latitude: post.latitude.and_then(|d| d.to_f64()),
            longitude: post.longitude.and_then(|d| d.to_f64()),
            created_at: post.created_at.to_rfc3339(),
            updated_at: post.updated_at.to_rfc3339(),
        }
    }
}
