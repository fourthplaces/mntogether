use crate::common::PostId;
use crate::domains::posts::models::Post;
use crate::domains::tag::models::Tag;
use crate::domains::tag::TagData;
use crate::server::graphql::context::GraphQLContext;
use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL type for listing
#[derive(Debug, Clone)]
pub struct PostType {
    pub id: Uuid,
    pub organization_name: String,
    pub title: String,
    pub tldr: Option<String>,
    pub description: String,
    pub description_markdown: Option<String>,
    pub post_type: String,
    pub category: String,
    pub status: PostStatusData,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub source_url: Option<String>,
    pub website_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub business_info: Option<BusinessInfo>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl PostType {
    fn id(&self) -> Uuid {
        self.id
    }
    fn organization_name(&self) -> &str {
        &self.organization_name
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn tldr(&self) -> Option<&str> {
        self.tldr.as_deref()
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn description_markdown(&self) -> Option<&str> {
        self.description_markdown.as_deref()
    }
    fn post_type(&self) -> &str {
        &self.post_type
    }
    fn category(&self) -> &str {
        &self.category
    }
    fn status(&self) -> PostStatusData {
        self.status
    }
    fn urgency(&self) -> Option<&str> {
        self.urgency.as_deref()
    }
    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
    fn submission_type(&self) -> Option<&str> {
        self.submission_type.as_deref()
    }
    fn source_url(&self) -> Option<&str> {
        self.source_url.as_deref()
    }
    fn website_id(&self) -> Option<Uuid> {
        self.website_id
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn business_info(&self) -> Option<&BusinessInfo> {
        self.business_info.as_ref()
    }

    /// Get all tags for this listing
    async fn tags(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        let post_id = PostId::from_uuid(self.id);
        let tags = Tag::find_for_post(post_id, &context.db_pool).await?;
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

impl From<Post> for PostType {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.into_uuid(),
            organization_name: post.organization_name,
            title: post.title,
            tldr: post.tldr,
            description: post.description,
            description_markdown: post.description_markdown,
            post_type: post.post_type,
            category: post.category,
            status: match post.status.as_str() {
                "pending_approval" => PostStatusData::PendingApproval,
                "active" => PostStatusData::Active,
                "rejected" => PostStatusData::Rejected,
                "expired" => PostStatusData::Expired,
                "filled" => PostStatusData::Filled,
                _ => PostStatusData::PendingApproval, // default fallback
            },
            urgency: post.urgency,
            location: post.location,
            submission_type: post.submission_type,
            source_url: post.source_url,
            website_id: post.website_id.map(|id| id.into_uuid()),
            created_at: post.created_at,
            business_info: None, // Populated by query layer when post_type = 'business'
        }
    }
}

/// Contact information for GraphQL output
/// This is a thin wrapper over common::ContactInfo for GraphQL serialization.
#[derive(Debug, Clone, GraphQLObject, Serialize, Deserialize)]
pub struct ContactInfoGraphQL {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

impl From<crate::common::ContactInfo> for ContactInfoGraphQL {
    fn from(c: crate::common::ContactInfo) -> Self {
        Self {
            phone: c.phone,
            email: c.email,
            website: c.website,
        }
    }
}

/// Listing status for GraphQL
#[derive(Debug, Clone, Copy, GraphQLEnum)]
pub enum PostStatusData {
    PendingApproval,
    Active,
    Rejected,
    Expired,
    Filled,
}

/// Input for editing a listing before approval
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct EditPostInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

/// Input for user-submitted listings
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct SubmitPostInput {
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
    pub new_posts_count: i32,
    pub changed_posts_count: i32,
    pub disappeared_posts_count: i32,
}

/// Result of starting an async scrape job
#[derive(Debug, Clone, GraphQLObject)]
pub struct ScrapeJobResult {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Edge containing a post and its cursor (Relay spec)
#[derive(Debug, Clone)]
pub struct PostEdge {
    pub node: PostType,
    pub cursor: String,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl PostEdge {
    /// The post at the end of the edge
    fn node(&self) -> &PostType {
        &self.node
    }
    /// A cursor for pagination
    fn cursor(&self) -> &str {
        &self.cursor
    }
}

/// Connection type for paginated posts (Relay spec)
#[derive(Debug, Clone)]
pub struct PostConnection {
    pub edges: Vec<PostEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl PostConnection {
    /// A list of edges (post + cursor pairs)
    fn edges(&self) -> &[PostEdge] {
        &self.edges
    }
    /// Information about pagination
    fn page_info(&self) -> &crate::common::PageInfo {
        &self.page_info
    }
    /// Total count of posts matching the filter
    fn total_count(&self) -> i32 {
        self.total_count
    }
    /// Convenience: direct access to nodes (for simpler queries)
    fn nodes(&self) -> Vec<&PostType> {
        self.edges.iter().map(|e| &e.node).collect()
    }
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

/// Result of reposting a listing
#[derive(Debug, Clone)]
pub struct RepostResult {
    pub post: super::PostData,
    pub message: String,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl RepostResult {
    fn post(&self) -> &super::PostData {
        &self.post
    }
    fn message(&self) -> &str {
        &self.message
    }
}
