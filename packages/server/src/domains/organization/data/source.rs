use crate::common::{DomainId, SourceId};
use crate::domains::listings::data::ListingData;
use crate::domains::listings::models::listing::Listing;
use crate::domains::organization::models::source::OrganizationSource;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of a domain source (decoupled from organizations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceData {
    pub id: String,
    pub source_url: String, // Maps to domain_url in database
    pub last_scraped_at: Option<String>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub created_at: String,
}

impl From<OrganizationSource> for SourceData {
    fn from(source: OrganizationSource) -> Self {
        Self {
            id: source.id.to_string(),
            source_url: source.domain_url, // Map domain_url to source_url for frontend compatibility
            last_scraped_at: source.last_scraped_at.map(|dt| dt.to_rfc3339()),
            scrape_frequency_hours: source.scrape_frequency_hours,
            active: source.active,
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

    fn last_scraped_at(&self) -> Option<String> {
        self.last_scraped_at.clone()
    }

    fn scrape_frequency_hours(&self) -> i32 {
        self.scrape_frequency_hours
    }

    fn active(&self) -> bool {
        self.active
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    /// Get all listings scraped from this source
    async fn listings(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<ListingData>> {
        let uuid = Uuid::parse_str(&self.id)?;
        let source_id = SourceId::from_uuid(uuid);
        let domain_id = DomainId::from_uuid(source_id.into_uuid());
        let listings = Listing::find_by_domain_id(domain_id, &context.db_pool).await?;
        Ok(listings.into_iter().map(ListingData::from).collect())
    }
}
