use crate::domains::organization::data::NeedData;
use crate::domains::organization::models::need::OrganizationNeed;
use crate::domains::organization::models::source::OrganizationSource;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of an organization source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceData {
    pub id: String,
    pub organization_name: String,
    pub source_url: String,
    pub last_scraped_at: Option<String>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub created_at: String,
}

impl From<OrganizationSource> for SourceData {
    fn from(source: OrganizationSource) -> Self {
        Self {
            id: source.id.to_string(),
            organization_name: source.organization_name,
            source_url: source.source_url,
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

    fn organization_name(&self) -> String {
        self.organization_name.clone()
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

    /// Get all needs scraped from this source
    async fn needs(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<NeedData>> {
        let source_id = Uuid::parse_str(&self.id)?;
        let needs = OrganizationNeed::find_by_source_id(source_id, &context.db_pool).await?;
        Ok(needs.into_iter().map(NeedData::from).collect())
    }
}
