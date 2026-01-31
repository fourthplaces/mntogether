use crate::domains::listings::data::ListingData;
use crate::domains::listings::models::listing::Listing;
use crate::domains::organization::models::{Post, PostStatus};
use crate::server::graphql::GraphQLContext;
use chrono::{DateTime, Utc};
use juniper::{FieldResult, GraphQLEnum, GraphQLInputObject, GraphQLObject};
use uuid::Uuid;

/// GraphQL type for post
#[derive(Debug, Clone)]
pub struct PostData {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub status: PostStatusData,
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

#[juniper::graphql_object(context = GraphQLContext)]
impl PostData {
    fn id(&self) -> Uuid {
        self.id
    }

    fn listing_id(&self) -> Uuid {
        self.listing_id
    }

    fn status(&self) -> PostStatusData {
        self.status
    }

    fn published_at(&self) -> Option<DateTime<Utc>> {
        self.published_at
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    fn custom_title(&self) -> Option<String> {
        self.custom_title.clone()
    }

    fn custom_description(&self) -> Option<String> {
        self.custom_description.clone()
    }

    fn custom_tldr(&self) -> Option<String> {
        self.custom_tldr.clone()
    }

    fn outreach_copy(&self) -> Option<String> {
        self.outreach_copy.clone()
    }

    fn view_count(&self) -> i32 {
        self.view_count
    }

    fn click_count(&self) -> i32 {
        self.click_count
    }

    fn response_count(&self) -> i32 {
        self.response_count
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Fetch the associated listing
    async fn listing(&self, ctx: &GraphQLContext) -> FieldResult<ListingData> {
        let listing = Listing::find_by_id(
            crate::common::ListingId::from_uuid(self.listing_id),
            &ctx.db_pool,
        )
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch listing: {}", e),
                juniper::Value::null(),
            )
        })?;

        Ok(ListingData::from(listing))
    }
}

impl From<Post> for PostData {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.into_uuid(),
            listing_id: post.listing_id.into_uuid(),
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
pub enum PostStatusData {
    Draft,
    Published,
    Expired,
    Archived,
}

impl From<PostStatus> for PostStatusData {
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
    pub listing_id: Uuid,
    pub custom_title: Option<String>,
    pub custom_description: Option<String>,
    pub custom_tldr: Option<String>,
    pub targeting_hints: Option<String>, // JSON string, parsed in resolver
    pub expires_in_days: Option<i32>,
}

/// Result of reposting a listing
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = GraphQLContext)]
pub struct RepostResult {
    pub post: PostData,
    pub message: String,
}
