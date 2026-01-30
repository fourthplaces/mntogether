use crate::common::OrganizationId;
use crate::domains::organization::data::SourceData;
use crate::domains::organization::models::Organization;
use crate::kernel::tag::Tag;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};

/// GraphQL-friendly representation of an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub domain_id: Option<String>,
    pub contact_info: Option<ContactInfo>,
    pub location: Option<String>,
    pub verified: bool,
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
        let contact_info = Some(ContactInfo {
            email: org.email,
            phone: org.phone,
            website: org.website,
        });

        Self {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            domain_id: org.domain_id.map(|d| d.to_string()),
            contact_info,
            location: org.primary_address,
            verified: org.verified,
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

    /// Get domain source for this organization (if linked to a domain)
    async fn sources(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<SourceData>> {
        use crate::domains::organization::models::OrganizationSource;
        use crate::common::SourceId;

        // Organizations are now linked to domains, not sources directly
        // If organization has a domain_id, return that domain source
        if let Some(domain_id_str) = &self.domain_id {
            if let Ok(uuid) = uuid::Uuid::parse_str(domain_id_str) {
                let source_id = SourceId::from_uuid(uuid);
                if let Ok(source) = OrganizationSource::find_by_id(source_id, &context.db_pool).await {
                    return Ok(vec![SourceData::from(source)]);
                }
            }
        }

        // No domain linked
        Ok(vec![])
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
