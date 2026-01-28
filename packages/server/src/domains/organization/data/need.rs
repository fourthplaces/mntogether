use crate::domains::organization::data::SourceData;
use crate::domains::organization::models::need::OrganizationNeed;
use crate::domains::organization::models::source::OrganizationSource;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of an organization need
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedData {
    pub id: String,
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub contact_info: Option<ContactInfoData>,
    pub urgency: Option<String>,
    pub status: String,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub submitted_by_volunteer_id: Option<String>,
    pub source_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfoData {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

impl From<OrganizationNeed> for NeedData {
    fn from(need: OrganizationNeed) -> Self {
        let contact_info = need
            .contact_info
            .and_then(|json| serde_json::from_value::<ContactInfoData>(json).ok());

        Self {
            id: need.id.to_string(),
            organization_name: need.organization_name,
            title: need.title,
            description: need.description,
            description_markdown: need.description_markdown,
            tldr: need.tldr,
            contact_info,
            urgency: need.urgency,
            status: need.status,
            location: need.location,
            submission_type: need.submission_type,
            submitted_by_volunteer_id: need.submitted_by_volunteer_id.map(|id| id.to_string()),
            source_id: need.source_id.map(|id| id.to_string()),
            created_at: need.created_at.to_rfc3339(),
            updated_at: need.updated_at.to_rfc3339(),
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl NeedData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn organization_name(&self) -> String {
        self.organization_name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn description_markdown(&self) -> Option<String> {
        self.description_markdown.clone()
    }

    fn tldr(&self) -> Option<String> {
        self.tldr.clone()
    }

    fn contact_info(&self) -> Option<ContactInfoData> {
        self.contact_info.clone()
    }

    fn urgency(&self) -> Option<String> {
        self.urgency.clone()
    }

    fn status(&self) -> String {
        self.status.clone()
    }

    fn location(&self) -> Option<String> {
        self.location.clone()
    }

    fn submission_type(&self) -> Option<String> {
        self.submission_type.clone()
    }

    fn submitted_by_volunteer_id(&self) -> Option<String> {
        self.submitted_by_volunteer_id.clone()
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    fn updated_at(&self) -> String {
        self.updated_at.clone()
    }

    /// Get the source this need was scraped from (if applicable)
    async fn source(&self, context: &GraphQLContext) -> juniper::FieldResult<Option<SourceData>> {
        let Some(source_id_str) = &self.source_id else {
            return Ok(None);
        };

        let source_id = Uuid::parse_str(source_id_str)?;
        let source = OrganizationSource::find_by_id(source_id, &context.db_pool).await?;
        Ok(Some(source.into()))
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ContactInfoData {
    fn email(&self) -> Option<String> {
        self.email.clone()
    }

    fn phone(&self) -> Option<String> {
        self.phone.clone()
    }

    fn website(&self) -> Option<String> {
        self.website.clone()
    }
}

/// Paginated needs response
#[derive(Debug, Clone)]
pub struct NeedsConnection {
    pub nodes: Vec<NeedData>,
    pub total_count: i64,
    pub has_next_page: bool,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl NeedsConnection {
    fn nodes(&self) -> Vec<NeedData> {
        self.nodes.clone()
    }

    fn total_count(&self) -> i32 {
        self.total_count as i32
    }

    fn has_next_page(&self) -> bool {
        self.has_next_page
    }
}
