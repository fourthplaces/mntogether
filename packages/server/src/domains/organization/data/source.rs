use crate::common::WebsiteId;
use crate::domains::listings::data::ListingData;
use crate::domains::listings::models::listing::Listing;
use crate::domains::scraping::models::Website;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of a website source (decoupled from organizations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceData {
    pub id: String,
    pub source_url: String,  // Maps to website_url in database
    pub website_url: String, // New canonical field name
    pub last_scraped_at: Option<String>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub status: String, // Website approval status
    pub submitted_by: Option<String>,
    pub submitter_type: Option<String>,
    pub agent_id: Option<String>, // Agent that discovered this website
    pub created_at: String,
}

impl From<Website> for SourceData {
    fn from(source: Website) -> Self {
        Self {
            id: source.id.to_string(),
            source_url: source.url.clone(), // Map url to source_url for frontend compatibility
            website_url: source.url,        // Also expose as websiteUrl
            last_scraped_at: source.last_scraped_at.map(|dt| dt.to_rfc3339()),
            scrape_frequency_hours: source.scrape_frequency_hours,
            active: source.active,
            status: source.status,
            submitted_by: source.submitted_by.map(|id| id.to_string()),
            submitter_type: source.submitter_type,
            agent_id: source.agent_id.map(|id| id.to_string()),
            created_at: source.created_at.to_rfc3339(),
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl SourceData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn source_url(&self) -> String {
        self.source_url.clone()
    }

    fn website_url(&self) -> String {
        self.website_url.clone()
    }

    fn last_scraped_at(&self) -> Option<String> {
        self.last_scraped_at.clone()
    }

    fn scrape_frequency_hours(&self) -> i32 {
        self.scrape_frequency_hours
    }

    fn active(&self) -> bool {
        self.active
    }

    fn status(&self) -> String {
        self.status.clone()
    }

    fn submitted_by(&self) -> Option<String> {
        self.submitted_by.clone()
    }

    fn submitter_type(&self) -> Option<String> {
        self.submitter_type.clone()
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    fn agent_id(&self) -> Option<String> {
        self.agent_id.clone()
    }

    /// Get count of website snapshots (submitted pages)
    async fn snapshots_count(&self, context: &GraphQLContext) -> juniper::FieldResult<i32> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM website_snapshots WHERE website_id = $1",
        )
        .bind(website_id)
        .fetch_one(&context.db_pool)
        .await?;
        Ok(count as i32)
    }

    /// Get count of listings from this website
    async fn listings_count(&self, context: &GraphQLContext) -> juniper::FieldResult<i32> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM listings WHERE website_id = $1")
                .bind(website_id)
                .fetch_one(&context.db_pool)
                .await?;
        Ok(count as i32)
    }

    /// Get all listings scraped from this source
    async fn listings(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<ListingData>> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let listings = Listing::find_by_website_id(website_id, &context.db_pool).await?;
        Ok(listings.into_iter().map(ListingData::from).collect())
    }
}
