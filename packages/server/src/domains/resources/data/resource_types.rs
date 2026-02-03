//! GraphQL data types for resources

use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use uuid::Uuid;

use crate::common::ResourceId;
use crate::domains::contacts::ContactData;
use crate::domains::resources::models::{Resource, ResourceSource, ResourceVersion};
use crate::domains::tag::TagData;
use crate::server::graphql::context::GraphQLContext;

/// Resource status for GraphQL
#[derive(Debug, Clone, Copy, GraphQLEnum)]
pub enum ResourceStatusData {
    PendingApproval,
    Active,
    Rejected,
    Expired,
}

impl From<&str> for ResourceStatusData {
    fn from(s: &str) -> Self {
        match s {
            "pending_approval" => ResourceStatusData::PendingApproval,
            "active" => ResourceStatusData::Active,
            "rejected" => ResourceStatusData::Rejected,
            "expired" => ResourceStatusData::Expired,
            _ => ResourceStatusData::PendingApproval,
        }
    }
}

/// GraphQL type for Resource
#[derive(Debug, Clone)]
pub struct ResourceData {
    pub id: Uuid,
    pub website_id: Uuid,
    pub title: String,
    pub content: String,
    pub location: Option<String>,
    pub status: ResourceStatusData,
    pub organization_name: Option<String>,
    pub has_embedding: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ResourceData {
    fn id(&self) -> Uuid {
        self.id
    }

    fn website_id(&self) -> Uuid {
        self.website_id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn content(&self) -> &str {
        &self.content
    }

    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    fn status(&self) -> ResourceStatusData {
        self.status
    }

    fn organization_name(&self) -> Option<&str> {
        self.organization_name.as_deref()
    }

    fn has_embedding(&self) -> bool {
        self.has_embedding
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Get contacts for this resource
    async fn contacts(&self, ctx: &GraphQLContext) -> juniper::FieldResult<Vec<ContactData>> {
        use crate::domains::contacts::Contact;

        let resource_id = ResourceId::from_uuid(self.id);
        let contacts = Contact::find_for_resource(resource_id, &ctx.db_pool).await?;
        Ok(contacts.into_iter().map(ContactData::from).collect())
    }

    /// Get source URLs for this resource
    async fn source_urls(&self, ctx: &GraphQLContext) -> juniper::FieldResult<Vec<String>> {
        let resource_id = ResourceId::from_uuid(self.id);
        let urls = ResourceSource::find_urls_by_resource_id(resource_id, &ctx.db_pool).await?;
        Ok(urls)
    }

    /// Get tags for this resource
    async fn tags(&self, ctx: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        use crate::domains::resources::models::ResourceTag;

        let resource_id = ResourceId::from_uuid(self.id);
        let tags = ResourceTag::find_tags_for_resource(resource_id, &ctx.db_pool).await?;
        Ok(tags.into_iter().map(TagData::from).collect())
    }

    /// Get version history for this resource
    async fn versions(
        &self,
        ctx: &GraphQLContext,
    ) -> juniper::FieldResult<Vec<ResourceVersionData>> {
        let resource_id = ResourceId::from_uuid(self.id);
        let versions = ResourceVersion::find_by_resource_id(resource_id, &ctx.db_pool).await?;
        Ok(versions
            .into_iter()
            .map(ResourceVersionData::from)
            .collect())
    }

    /// Get the number of versions for this resource
    async fn version_count(&self, ctx: &GraphQLContext) -> juniper::FieldResult<i32> {
        let resource_id = ResourceId::from_uuid(self.id);
        let count = ResourceVersion::count_by_resource_id(resource_id, &ctx.db_pool).await?;
        Ok(count as i32)
    }
}

impl From<Resource> for ResourceData {
    fn from(r: Resource) -> Self {
        Self {
            id: r.id.into_uuid(),
            website_id: r.website_id.into_uuid(),
            title: r.title,
            content: r.content,
            location: r.location,
            status: ResourceStatusData::from(r.status.as_str()),
            organization_name: r.organization_name,
            has_embedding: r.embedding.is_some(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// GraphQL type for ResourceVersion (audit trail)
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = GraphQLContext)]
pub struct ResourceVersionData {
    pub id: Uuid,
    pub resource_id: Uuid,
    pub title: String,
    pub content: String,
    pub location: Option<String>,
    pub change_reason: String,
    pub created_at: DateTime<Utc>,
}

impl From<ResourceVersion> for ResourceVersionData {
    fn from(v: ResourceVersion) -> Self {
        Self {
            id: v.id.into_uuid(),
            resource_id: v.resource_id.into_uuid(),
            title: v.title,
            content: v.content,
            location: v.location,
            change_reason: v.change_reason,
            created_at: v.created_at,
        }
    }
}

/// Edge containing a resource and its cursor (Relay spec)
#[derive(Debug, Clone)]
pub struct ResourceEdge {
    pub node: ResourceData,
    pub cursor: String,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ResourceEdge {
    /// The resource at the end of the edge
    fn node(&self) -> &ResourceData {
        &self.node
    }
    /// A cursor for pagination
    fn cursor(&self) -> &str {
        &self.cursor
    }
}

/// Connection type for paginated resources (Relay spec)
#[derive(Debug, Clone)]
pub struct ResourceConnection {
    pub edges: Vec<ResourceEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ResourceConnection {
    /// A list of edges (resource + cursor pairs)
    fn edges(&self) -> &[ResourceEdge] {
        &self.edges
    }
    /// Information about pagination
    fn page_info(&self) -> &crate::common::PageInfo {
        &self.page_info
    }
    /// Total count of resources matching the filter
    fn total_count(&self) -> i32 {
        self.total_count
    }
    /// Convenience: direct access to nodes (for simpler queries)
    fn nodes(&self) -> Vec<&ResourceData> {
        self.edges.iter().map(|e| &e.node).collect()
    }
}

/// Input for editing a resource
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct EditResourceInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub location: Option<String>,
}
