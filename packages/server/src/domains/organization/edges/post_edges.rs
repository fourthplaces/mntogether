use super::post_types::{CreatePostInput, PostGql, RepostResult};
use crate::domains::organization::models::{OrganizationNeed, Post};
use crate::server::graphql::context::GraphQLContext;
use juniper::FieldResult;
use uuid::Uuid;

/// Query published posts (for volunteers)
pub async fn query_published_posts(
    ctx: &GraphQLContext,
    limit: Option<i32>,
) -> FieldResult<Vec<PostGql>> {
    let posts = Post::find_published(limit.map(|l| l as i64), &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch published posts: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(posts.into_iter().map(PostGql::from).collect())
}

/// Query posts for a specific need
pub async fn query_posts_for_need(
    ctx: &GraphQLContext,
    need_id: Uuid,
) -> FieldResult<Vec<PostGql>> {
    let posts = Post::find_by_need_id(need_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch posts for need: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(posts.into_iter().map(PostGql::from).collect())
}

/// Query a single post by ID
pub async fn query_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<Option<PostGql>> {
    let post = Post::find_by_id(post_id, &ctx.db_pool).await.map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to fetch post: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(post.map(PostGql::from))
}

/// Create a custom post for a need (admin only)
pub async fn create_custom_post(
    ctx: &GraphQLContext,
    input: CreatePostInput,
) -> FieldResult<PostGql> {
    // Require admin access
    ctx.require_admin()?;

    // Verify need exists and is active (validation in model)
    let need = OrganizationNeed::find_by_id(input.need_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(format!("Need not found: {}", e), juniper::Value::null())
        })?;

    // Use model validation method
    need.ensure_active()
        .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;

    // Get admin user ID from context
    let created_by = ctx.auth_user.as_ref().map(|u| u.member_id);

    // Create post
    let post = Post::create_and_publish_custom(
        input.need_id,
        created_by,
        input.custom_title,
        input.custom_description,
        input.custom_tldr,
        input
            .targeting_hints
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        input.expires_in_days.map(|d| d as i64),
        &ctx.db_pool,
    )
    .await
    .map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to create post: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(PostGql::from(post))
}

/// Repost a need (create new post for existing active need) (admin only)
pub async fn repost_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<RepostResult> {
    // Require admin access
    ctx.require_admin()?;

    // Verify need exists and is active (validation in model)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(format!("Need not found: {}", e), juniper::Value::null())
        })?;

    // Use model validation method
    need.ensure_active()
        .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;

    // Get admin user ID from context
    let created_by = ctx.auth_user.as_ref().map(|u| u.member_id);

    // Create new post
    let post = Post::create_and_publish(need_id, created_by, None, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to repost need: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(RepostResult {
        post: PostGql::from(post),
        message: format!("Need reposted successfully. Post expires in 5 days."),
    })
}

/// Expire a post (admin only)
pub async fn expire_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostGql> {
    // Require admin access
    ctx.require_admin()?;

    let post = Post::expire(post_id, &ctx.db_pool).await.map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to expire post: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(PostGql::from(post))
}

/// Archive a post (admin only)
pub async fn archive_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostGql> {
    // Require admin access
    ctx.require_admin()?;

    let post = Post::archive(post_id, &ctx.db_pool).await.map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to archive post: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(PostGql::from(post))
}

/// Increment post view count (public - called when volunteer sees post)
pub async fn increment_post_view(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    Post::increment_view_count(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to increment view count: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(true)
}

/// Increment post click count (public - called when volunteer clicks post)
pub async fn increment_post_click(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    Post::increment_click_count(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to increment click count: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(true)
}

/// Increment post response count (public - called when volunteer responds to post)
pub async fn increment_post_response(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    Post::increment_response_count(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to increment response count: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(true)
}
