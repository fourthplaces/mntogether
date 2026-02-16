use crate::domains::posts::models::Post;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Post type for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostType {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub description: String,
    pub description_markdown: Option<String>,
    pub post_type: String,
    pub category: String,
    pub status: PostStatusData,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub business_info: Option<BusinessInfo>,
}

/// Business-specific information for cause-driven commerce
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            title: post.title,
            summary: post.summary,
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
            created_at: post.created_at,
            published_at: post.published_at,
            business_info: None, // Populated by query layer when post_type = 'business'
        }
    }
}

/// A post with distance info from proximity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyPostType {
    pub post: PostType,
    pub distance_miles: f64,
    pub zip_code: Option<String>,
    pub city: Option<String>,
}

/// Post status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStatusData {
    PendingApproval,
    Active,
    Rejected,
    Expired,
    Filled,
}

impl std::fmt::Display for PostStatusData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostStatusData::PendingApproval => write!(f, "pending_approval"),
            PostStatusData::Active => write!(f, "active"),
            PostStatusData::Rejected => write!(f, "rejected"),
            PostStatusData::Expired => write!(f, "expired"),
            PostStatusData::Filled => write!(f, "filled"),
        }
    }
}

/// Input for editing a listing before approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPostInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub description_markdown: Option<String>,
    pub summary: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

/// Input for user-submitted listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPostInput {
    pub title: String,
    pub description: String,
    pub contact_info: Option<ContactInfoInput>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfoInput {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Result of scraping an organization source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub source_id: Uuid,
    pub new_posts_count: i32,
    pub changed_posts_count: i32,
    pub disappeared_posts_count: i32,
}

/// Result of starting an async scrape job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeJobResult {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Edge containing a post and its cursor (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostEdge {
    pub node: PostType,
    pub cursor: String,
}

/// Connection type for paginated posts (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostConnection {
    pub edges: Vec<PostEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}

/// Input for submitting a resource link from the public
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResourceLinkInput {
    pub url: String,
    pub context: Option<String>,
    pub submitter_contact: Option<String>,
}

/// Result of submitting a resource link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResourceLinkResult {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}

/// Result of reposting a listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepostResult {
    pub post: super::PostData,
    pub message: String,
}
