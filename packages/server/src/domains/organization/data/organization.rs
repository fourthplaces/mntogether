use crate::domains::organization::data::SourceData;
use crate::domains::organization::models::{Organization, Tag};
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub contact_info: Option<ContactInfo>,
    pub location: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagData {
    pub id: String,
    pub kind: String,
    pub value: String,
}

impl From<Organization> for OrganizationData {
    fn from(org: Organization) -> Self {
        let contact_info = org.contact_info.and_then(|json| {
            serde_json::from_value::<ContactInfo>(json).ok()
        });

        Self {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            contact_info,
            location: org.location,
            city: org.city,
            state: org.state,
            status: org.status,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        }
    }
}

impl From<Tag> for TagData {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id.to_string(),
            kind: tag.kind,
            value: tag.value,
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

    fn contact_info(&self) -> Option<ContactInfo> {
        self.contact_info.clone()
    }

    fn location(&self) -> Option<String> {
        self.location.clone()
    }

    fn city(&self) -> Option<String> {
        self.city.clone()
    }

    fn state(&self) -> Option<String> {
        self.state.clone()
    }

    fn status(&self) -> String {
        self.status.clone()
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    fn updated_at(&self) -> String {
        self.updated_at.clone()
    }

    /// Get tags for this organization
    async fn tags(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        let org_id = Uuid::parse_str(&self.id)?;
        let tags = Tag::find_for_organization(org_id, &context.db_pool).await?;
        Ok(tags.into_iter().map(TagData::from).collect())
    }

    /// Get sources for this organization
    async fn sources(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<SourceData>> {
        use crate::domains::organization::models::OrganizationSource;

        let sources = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM organization_sources WHERE organization_id = $1 ORDER BY created_at DESC"
        )
        .bind(Uuid::parse_str(&self.id)?)
        .fetch_all(&context.db_pool)
        .await?;

        Ok(sources.into_iter().map(SourceData::from).collect())
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ContactInfo {
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

#[juniper::graphql_object(Context = GraphQLContext)]
impl TagData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn kind(&self) -> String {
        self.kind.clone()
    }

    fn value(&self) -> String {
        self.value.clone()
    }
}
