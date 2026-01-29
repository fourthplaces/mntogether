use crate::domains::listings::models::Listing;
use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL type for listing
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "A service, opportunity, or business listing")]
pub struct ListingType {
    pub id: Uuid,
    pub organization_name: String,
    pub title: String,
    pub tldr: Option<String>,
    pub description: String,
    pub description_markdown: Option<String>,
    pub listing_type: String,
    pub category: String,
    pub status: ListingStatusData,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,

    // Business-specific fields (only populated when listing_type = 'business')
    pub business_info: Option<BusinessInfo>,
}

/// Business-specific information for cause-driven commerce
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "Business listing details including cause-driven commerce")]
pub struct BusinessInfo {
    pub accepts_donations: bool,
    pub donation_link: Option<String>,
    pub gift_cards_available: bool,
    pub gift_card_link: Option<String>,
    pub online_ordering_link: Option<String>,
    pub delivery_available: bool,

    // Cause-driven commerce
    pub proceeds_percentage: Option<f64>,
    pub proceeds_beneficiary_id: Option<Uuid>,
    pub proceeds_description: Option<String>,
    pub impact_statement: Option<String>,
}

impl From<Listing> for ListingType {
    fn from(listing: Listing) -> Self {
        Self {
            id: listing.id.into_uuid(),
            organization_name: listing.organization_name,
            title: listing.title,
            tldr: listing.tldr,
            description: listing.description,
            description_markdown: listing.description_markdown,
            listing_type: listing.listing_type,
            category: listing.category,
            status: match listing.status.as_str() {
                "pending_approval" => ListingStatusData::PendingApproval,
                "active" => ListingStatusData::Active,
                "rejected" => ListingStatusData::Rejected,
                "expired" => ListingStatusData::Expired,
                "filled" => ListingStatusData::Filled,
                _ => ListingStatusData::PendingApproval, // default fallback
            },
            urgency: listing.urgency,
            location: listing.location,
            submission_type: listing.submission_type,
            source_url: listing.source_url,
            created_at: listing.created_at,
            business_info: None, // TODO: Populate from business_listings table when listing_type = 'business'
        }
    }
}

/// Contact information
#[derive(Debug, Clone, GraphQLObject, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Listing status for GraphQL
#[derive(Debug, Clone, Copy, GraphQLEnum)]
pub enum ListingStatusData {
    PendingApproval,
    Active,
    Rejected,
    Expired,
    Filled,
}

/// Input for editing a listing before approval
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct EditListingInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

/// Input for user-submitted listings
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct SubmitListingInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub contact_info: Option<ContactInfoInput>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, GraphQLInputObject, Serialize, Deserialize)]
pub struct ContactInfoInput {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Result of scraping an organization source
#[derive(Debug, Clone, GraphQLObject)]
pub struct ScrapeResult {
    pub source_id: Uuid,
    pub new_listings_count: i32,
    pub changed_listings_count: i32,
    pub disappeared_listings_count: i32,
}

/// Result of starting an async scrape job
#[derive(Debug, Clone, GraphQLObject)]
pub struct ScrapeJobResult {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Connection type for paginated listings
#[derive(Debug, Clone, GraphQLObject)]
pub struct ListingConnection {
    pub nodes: Vec<ListingType>,
    pub total_count: i32,
    pub has_next_page: bool,
}

/// Input for submitting a resource link from the public
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct SubmitResourceLinkInput {
    pub url: String,
    pub context: Option<String>,
    pub submitter_contact: Option<String>,
}

/// Result of submitting a resource link
#[derive(Debug, Clone, GraphQLObject)]
pub struct SubmitResourceLinkResult {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}
