use crate::common::OrganizationId;
use crate::domains::organization::data::WebsiteData;
use crate::domains::organization::models::Organization;
use crate::domains::posts::data::types::ContactInfoGraphQL;
use crate::domains::tag::TagData;
use crate::kernel::tag::Tag;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};

/// GraphQL-friendly representation of an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub website_id: Option<String>,
    pub contact_info: Option<ContactInfoGraphQL>,
    pub location: Option<String>,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Organization> for OrganizationData {
    fn from(org: Organization) -> Self {
        let contact_info = Some(ContactInfoGraphQL {
            email: org.email,
            phone: org.phone,
            website: org.website,
        });

        Self {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            website_id: org.website_id.map(|d| d.to_string()),
            contact_info,
            location: org.primary_address,
            verified: org.verified,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl OrganizationData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn contact_info(&self) -> Option<ContactInfoGraphQL> {
        self.contact_info.clone()
    }

    fn location(&self) -> Option<String> {
        self.location.clone()
    }

    fn verified(&self) -> bool {
        self.verified
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    fn updated_at(&self) -> String {
        self.updated_at.clone()
    }

    /// Get tags for this organization
    async fn tags(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        let org_id = OrganizationId::parse(&self.id)?;
        let tags = Tag::find_for_organization(org_id, &context.db_pool).await?;
        Ok(tags.into_iter().map(TagData::from).collect())
    }

    /// Get the website linked to this organization (if any)
    async fn website(&self, context: &GraphQLContext) -> juniper::FieldResult<Option<WebsiteData>> {
        use crate::common::WebsiteId;
        use crate::domains::website::models::Website;

        if let Some(website_id_str) = &self.website_id {
            if let Ok(uuid) = uuid::Uuid::parse_str(website_id_str) {
                let website_id = WebsiteId::from_uuid(uuid);
                if let Ok(website) = Website::find_by_id(website_id, &context.db_pool).await {
                    return Ok(Some(WebsiteData::from(website)));
                }
            }
        }

        Ok(None)
    }
}

// ============================================================================
// Relay Pagination Types
// ============================================================================

/// Edge containing an organization and its cursor (Relay spec)
#[derive(Debug, Clone)]
pub struct OrganizationEdge {
    pub node: OrganizationData,
    pub cursor: String,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl OrganizationEdge {
    fn node(&self) -> &OrganizationData {
        &self.node
    }
    fn cursor(&self) -> &str {
        &self.cursor
    }
}

/// Connection type for paginated organizations (Relay spec)
#[derive(Debug, Clone)]
pub struct OrganizationConnection {
    pub edges: Vec<OrganizationEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl OrganizationConnection {
    fn edges(&self) -> &[OrganizationEdge] {
        &self.edges
    }
    fn page_info(&self) -> &crate::common::PageInfo {
        &self.page_info
    }
    fn total_count(&self) -> i32 {
        self.total_count
    }
    fn nodes(&self) -> Vec<&OrganizationData> {
        self.edges.iter().map(|e| &e.node).collect()
    }
}
