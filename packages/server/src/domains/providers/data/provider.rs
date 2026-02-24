use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domains::providers::models::Provider;

/// Provider data type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderData {
    pub id: String,

    // Profile
    pub name: String,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,

    // Links
    pub member_id: Option<String>,
    pub website_id: Option<String>,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,

    // Service modes
    pub offers_in_person: bool,
    pub offers_remote: bool,

    // Availability
    pub accepting_clients: bool,

    // Approval workflow
    pub status: String,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Provider> for ProviderData {
    fn from(provider: Provider) -> Self {
        Self {
            id: provider.id.to_string(),
            name: provider.name,
            bio: provider.bio,
            why_statement: provider.why_statement,
            headline: provider.headline,
            profile_image_url: provider.profile_image_url,
            member_id: provider.member_id.map(|id| id.to_string()),
            website_id: provider.website_id.map(|id| id.to_string()),
            location: provider.location,
            latitude: provider.latitude,
            longitude: provider.longitude,
            service_radius_km: provider.service_radius_km,
            offers_in_person: provider.offers_in_person,
            offers_remote: provider.offers_remote,
            accepting_clients: provider.accepting_clients,
            status: provider.status,
            reviewed_at: provider.reviewed_at,
            rejection_reason: provider.rejection_reason,
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}

/// Input for submitting a new provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitProviderInput {
    pub name: String,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,
    pub offers_in_person: Option<bool>,
    pub offers_remote: Option<bool>,
    pub accepting_clients: Option<bool>,
}

/// Input for updating a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProviderInput {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,
    pub offers_in_person: Option<bool>,
    pub offers_remote: Option<bool>,
    pub accepting_clients: Option<bool>,
}

/// Provider status for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusData {
    pub status: String,
    pub count: i32,
}

// ============================================================================
// Relay Pagination Types
// ============================================================================

/// Edge containing a provider and its cursor (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEdge {
    pub node: ProviderData,
    pub cursor: String,
}

/// Connection type for paginated providers (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConnection {
    pub edges: Vec<ProviderEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}
