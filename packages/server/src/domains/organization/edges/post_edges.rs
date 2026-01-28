use super::post_types::{CreatePostInput, PostData, RepostResult};
use crate::common::{NeedId, PostId};
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::{OrganizationNeed, Post};
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw::{dispatch_request, EnvelopeMatch};
use uuid::Uuid;

/// Query published posts (for volunteers)
///
/// This uses engagement-based rotation to ensure fair visibility:
/// - Fetches posts sorted by view_count and last_displayed_at
/// - Updates last_displayed_at for all returned posts to track when they were shown
/// - This creates a round-robin effect where under-engaged posts resurface
pub async fn query_published_posts(
    ctx: &GraphQLContext,
    limit: Option<i32>,
) -> FieldResult<Vec<PostData>> {
    let posts = Post::find_published(limit.map(|l| l as i64), &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch published posts: {}", e),
                juniper::Value::null(),
            )
        })?;

    // Update last_displayed_at for engagement tracking
    // This is done asynchronously and failures are logged but don't block the response
    if !posts.is_empty() {
        let post_ids: Vec<PostId> = posts.iter().map(|p| p.id).collect();
        tokio::spawn({
            let pool = ctx.db_pool.clone();
            async move {
                if let Err(e) = Post::mark_displayed(&post_ids, &pool).await {
                    tracing::warn!(error = %e, "Failed to update last_displayed_at for posts");
                }
            }
        });
    }

    Ok(posts.into_iter().map(PostData::from).collect())
}

/// Query posts for a specific need
pub async fn query_posts_for_need(
    ctx: &GraphQLContext,
    need_id: Uuid,
) -> FieldResult<Vec<PostData>> {
    let need_id = NeedId::from_uuid(need_id);
    let posts = Post::find_by_need_id(need_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch posts for need: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(posts.into_iter().map(PostData::from).collect())
}

/// Query a single post by ID
pub async fn query_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<Option<PostData>> {
    let post_id = PostId::from_uuid(post_id);
    let post = Post::find_by_id(post_id, &ctx.db_pool).await.map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to fetch post: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(post.map(PostData::from))
}

/// Create a custom post for a need (admin only)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn create_custom_post(
    ctx: &GraphQLContext,
    input: CreatePostInput,
) -> FieldResult<PostData> {
    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let need_id = NeedId::from_uuid(input.need_id);

    // Dispatch request event and await PostCreated fact event
    let (post_id, need_id) = dispatch_request(
        OrganizationEvent::CreateCustomPostRequested {
            need_id,
            custom_title: input.custom_title,
            custom_description: input.custom_description,
            custom_tldr: input.custom_tldr,
            targeting_hints: input
                .targeting_hints
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
            expires_in_days: input.expires_in_days.map(|d| d as i64),
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::PostCreated { post_id, need_id } => {
                    Some(Ok((*post_id, *need_id)))
                }
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to create custom post: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch post", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Repost a need (create new post for existing active need) (admin only)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn repost_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<RepostResult> {
    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let need_id = NeedId::from_uuid(need_id);

    // Dispatch request event and await PostCreated fact event
    let post_id = dispatch_request(
        OrganizationEvent::RepostNeedRequested {
            need_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::PostCreated {
                    post_id,
                    need_id: nid,
                } if *nid == need_id => Some(Ok(*post_id)),
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to repost need: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch post", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

    Ok(RepostResult {
        post: PostData::from(post),
        message: "Need reposted successfully. Post expires in 5 days.".to_string(),
    })
}

/// Expire a post (admin only)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn expire_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Dispatch request event and await PostExpired fact event
    dispatch_request(
        OrganizationEvent::ExpirePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::PostExpired { post_id: pid } if *pid == post_id => Some(Ok(())),
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to expire post: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch post", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Archive a post (admin only)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn archive_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Dispatch request event and await PostArchived fact event
    dispatch_request(
        OrganizationEvent::ArchivePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &OrganizationEvent| match e {
                OrganizationEvent::PostArchived { post_id: pid } if *pid == post_id => Some(Ok(())),
                OrganizationEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to archive post: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch post", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Track post view (public - called when member sees post)
/// Following seesaw pattern: emit event (fire-and-forget)
pub async fn track_post_view(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Emit analytics event (fire-and-forget, no need to wait)
    ctx.bus
        .emit(OrganizationEvent::PostViewedRequested { post_id });
    Ok(true)
}

/// Track post click (public - called when member clicks post)
/// Following seesaw pattern: emit event (fire-and-forget)
pub async fn track_post_click(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Emit analytics event (fire-and-forget, no need to wait)
    ctx.bus
        .emit(OrganizationEvent::PostClickedRequested { post_id });
    Ok(true)
}
