use crate::domains::posts::models::Post;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Post type for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostType {
    pub id: Uuid,
    pub title: String,
    pub body_raw: String,
    pub post_type: String,
    pub category: String,
    pub status: PostStatusData,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<Post> for PostType {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.into_uuid(),
            title: post.title,
            body_raw: post.body_raw,
            post_type: post.post_type,
            category: post.category,
            status: match post.status.as_str() {
                "draft" => PostStatusData::Draft,
                "pending_approval" => PostStatusData::PendingApproval,
                "active" => PostStatusData::Active,
                "rejected" => PostStatusData::Rejected,
                "expired" => PostStatusData::Expired,
                "filled" => PostStatusData::Filled,
                "archived" => PostStatusData::Archived,
                _ => PostStatusData::Active, // default fallback
            },
            urgency: post.urgency,
            location: post.location,
            submission_type: post.submission_type,
            source_url: post.source_url,
            created_at: post.created_at,
            published_at: post.published_at,
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
    Draft,
    PendingApproval, // Legacy — kept for backward compat
    Active,
    Rejected,
    Expired,
    Filled,
    Archived,
}

impl std::fmt::Display for PostStatusData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostStatusData::Draft => write!(f, "draft"),
            PostStatusData::PendingApproval => write!(f, "pending_approval"),
            PostStatusData::Active => write!(f, "active"),
            PostStatusData::Rejected => write!(f, "rejected"),
            PostStatusData::Expired => write!(f, "expired"),
            PostStatusData::Filled => write!(f, "filled"),
            PostStatusData::Archived => write!(f, "archived"),
        }
    }
}

/// Input for editing a listing before approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPostInput {
    pub title: Option<String>,
    pub body_raw: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

/// Input for user-submitted listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPostInput {
    pub title: String,
    pub body_raw: String,
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

/// Result of reposting a listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepostResult {
    pub post: super::PostData,
    pub message: String,
}
