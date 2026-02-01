use crate::common::PostId;
use crate::domains::posts::models::Post;
use crate::kernel::tag::Tag;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};

/// GraphQL-friendly representation of a listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostData {
    pub id: String,
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub tldr: Option<String>,

    // Hot path fields
    pub post_type: String,
    pub category: String,
    pub capacity_status: Option<String>,
    pub urgency: Option<String>,
    pub status: String,

    // Verification
    pub verified_at: Option<String>,

    // Language
    pub source_language: String,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Timestamps
    pub created_at: String,
    pub updated_at: String,

    // Source tracking
    pub source_url: Option<String>,
}

/// Tag data for GraphQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagData {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
}

/// Service-specific properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePostData {
    // Access & Requirements
    pub requires_identification: bool,
    pub requires_appointment: bool,
    pub walk_ins_accepted: bool,

    // Delivery Methods
    pub remote_available: bool,
    pub in_person_available: bool,
    pub home_visits_available: bool,

    // Accessibility
    pub wheelchair_accessible: bool,
    pub interpretation_available: bool,

    // Costs
    pub free_service: bool,
    pub sliding_scale_fees: bool,
    pub accepts_insurance: bool,

    // Hours
    pub evening_hours: bool,
    pub weekend_hours: bool,
}

impl From<Post> for PostData {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.to_string(),
            organization_name: post.organization_name,
            title: post.title,
            description: post.description,
            tldr: post.tldr,
            post_type: post.post_type,
            category: post.category,
            capacity_status: post.capacity_status,
            urgency: post.urgency,
            status: post.status,
            verified_at: post.verified_at.map(|dt| dt.to_rfc3339()),
            source_language: post.source_language,
            location: post.location,
            latitude: post.latitude,
            longitude: post.longitude,
            created_at: post.created_at.to_rfc3339(),
            updated_at: post.updated_at.to_rfc3339(),
            source_url: post.source_url,
        }
    }
}

impl From<Tag> for TagData {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id.to_string(),
            kind: tag.kind,
            value: tag.value,
            display_name: tag.display_name,
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl PostData {
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

    fn tldr(&self) -> Option<String> {
        self.tldr.clone()
    }

    fn post_type(&self) -> String {
        self.post_type.clone()
    }

    fn category(&self) -> String {
        self.category.clone()
    }

    fn capacity_status(&self) -> Option<String> {
        self.capacity_status.clone()
    }

    fn urgency(&self) -> Option<String> {
        self.urgency.clone()
    }

    fn status(&self) -> String {
        self.status.clone()
    }

    fn verified_at(&self) -> Option<String> {
        self.verified_at.clone()
    }

    fn source_language(&self) -> String {
        self.source_language.clone()
    }

    fn location(&self) -> Option<String> {
        self.location.clone()
    }

    fn latitude(&self) -> Option<f64> {
        self.latitude
    }

    fn longitude(&self) -> Option<f64> {
        self.longitude
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    fn updated_at(&self) -> String {
        self.updated_at.clone()
    }

    fn source_url(&self) -> Option<String> {
        self.source_url.clone()
    }

    /// Get tags for this listing
    async fn tags(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        let post_id = PostId::parse(&self.id)?;
        let tags = Tag::find_for_post(post_id, &context.db_pool).await?;
        Ok(tags.into_iter().map(TagData::from).collect())
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

    fn display_name(&self) -> Option<String> {
        self.display_name.clone()
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ServicePostData {
    fn requires_identification(&self) -> bool {
        self.requires_identification
    }

    fn requires_appointment(&self) -> bool {
        self.requires_appointment
    }

    fn walk_ins_accepted(&self) -> bool {
        self.walk_ins_accepted
    }

    fn remote_available(&self) -> bool {
        self.remote_available
    }

    fn in_person_available(&self) -> bool {
        self.in_person_available
    }

    fn home_visits_available(&self) -> bool {
        self.home_visits_available
    }

    fn wheelchair_accessible(&self) -> bool {
        self.wheelchair_accessible
    }

    fn interpretation_available(&self) -> bool {
        self.interpretation_available
    }

    fn free_service(&self) -> bool {
        self.free_service
    }

    fn sliding_scale_fees(&self) -> bool {
        self.sliding_scale_fees
    }

    fn accepts_insurance(&self) -> bool {
        self.accepts_insurance
    }

    fn evening_hours(&self) -> bool {
        self.evening_hours
    }

    fn weekend_hours(&self) -> bool {
        self.weekend_hours
    }
}
