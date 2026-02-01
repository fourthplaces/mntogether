use crate::common::ListingId;
use crate::domains::listings::models::Listing;
use crate::domains::tag::models::Tag;
use crate::domains::tag::TagData;
use crate::server::graphql::context::GraphQLContext;
use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL type for listing
#[derive(Debug, Clone)]
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
    pub business_info: Option<BusinessInfo>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ListingType {
    fn id(&self) -> Uuid { self.id }
    fn organization_name(&self) -> &str { &self.organization_name }
    fn title(&self) -> &str { &self.title }
    fn tldr(&self) -> Option<&str> { self.tldr.as_deref() }
    fn description(&self) -> &str { &self.description }
    fn description_markdown(&self) -> Option<&str> { self.description_markdown.as_deref() }
    fn listing_type(&self) -> &str { &self.listing_type }
    fn category(&self) -> &str { &self.category }
    fn status(&self) -> ListingStatusData { self.status }
    fn urgency(&self) -> Option<&str> { self.urgency.as_deref() }
    fn location(&self) -> Option<&str> { self.location.as_deref() }
    fn submission_type(&self) -> Option<&str> { self.submission_type.as_deref() }
    fn source_url(&self) -> Option<&str> { self.source_url.as_deref() }
    fn created_at(&self) -> DateTime<Utc> { self.created_at }
    fn business_info(&self) -> Option<&BusinessInfo> { self.business_info.as_ref() }

    /// Get all tags for this listing
    async fn tags(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        let listing_id = ListingId::from_uuid(self.id);
        let tags = Tag::find_for_listing(listing_id, &context.db_pool).await?;
        Ok(tags.into_iter().map(TagData::from).collect())
    }
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
            business_info: None, // Populated by query layer when listing_type = 'business'
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
#[derive(Debug, Clone)]
pub struct ListingConnection {
    pub nodes: Vec<ListingType>,
    pub total_count: i32,
    pub has_next_page: bool,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ListingConnection {
    fn nodes(&self) -> &[ListingType] { &self.nodes }
    fn total_count(&self) -> i32 { self.total_count }
    fn has_next_page(&self) -> bool { self.has_next_page }
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
