use crate::domains::organization::models::{Post, PostStatus};
use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// GraphQL type for post
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "A temporal announcement about a volunteer need")]
pub struct PostGql {
    pub id: Uuid,
    pub need_id: Uuid,
    pub status: PostStatusGql,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub custom_title: Option<String>,
    pub custom_description: Option<String>,
    pub custom_tldr: Option<String>,
    pub outreach_copy: Option<String>,
    pub view_count: i32,
    pub click_count: i32,
    pub response_count: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Post> for PostGql {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            need_id: post.need_id,
            status: post.status.into(),
            published_at: post.published_at,
            expires_at: post.expires_at,
            custom_title: post.custom_title,
            custom_description: post.custom_description,
            custom_tldr: post.custom_tldr,
            outreach_copy: post.outreach_copy,
            view_count: post.view_count,
            click_count: post.click_count,
            response_count: post.response_count,
            created_at: post.created_at,
        }
    }
}

/// Post status for GraphQL
#[derive(Debug, Clone, Copy, GraphQLEnum)]
pub enum PostStatusGql {
    Draft,
    Published,
    Expired,
    Archived,
}

impl From<PostStatus> for PostStatusGql {
    fn from(status: PostStatus) -> Self {
        match status {
            PostStatus::Draft => Self::Draft,
            PostStatus::Published => Self::Published,
            PostStatus::Expired => Self::Expired,
            PostStatus::Archived => Self::Archived,
        }
    }
}

/// Input for creating a custom post
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct CreatePostInput {
    pub need_id: Uuid,
    pub custom_title: Option<String>,
    pub custom_description: Option<String>,
    pub custom_tldr: Option<String>,
    pub targeting_hints: Option<String>, // JSON string, parsed in resolver
    pub expires_in_days: Option<i32>,
}

/// Result of reposting a need
#[derive(Debug, Clone, GraphQLObject)]
pub struct RepostResult {
    pub post: PostGql,
    pub message: String,
}
