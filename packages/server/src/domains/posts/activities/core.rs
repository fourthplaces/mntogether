//! Post CRUD actions - entry-point functions for post operations
//!
//! These are called from Restate virtual objects.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return plain data.

use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{MemberId, PostId};
use crate::domains::posts::data::{EditPostInput, SubmitPostInput};
use super::post_operations::{self, UpdateAndApprovePost};
use crate::kernel::ServerDeps;

/// Submit a post from user input (public, goes to pending_approval)
/// Returns the created PostId.
pub async fn submit_post(
    input: SubmitPostInput,
    member_id: Option<Uuid>,
    deps: &ServerDeps,
) -> Result<PostId> {
    info!(title = %input.title, member_id = ?member_id, "Submitting user post");

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());
    let member_id_typed = member_id.map(MemberId::from_uuid);

    let post = post_operations::create_post(
        member_id_typed,
        input.title.clone(),
        input.description,
        contact_json,
        input.urgency,
        input.location,
        None, // ip_address
        "user_submitted".to_string(),
        None, // source_type
        None, // source_id
        deps.ai.as_ref(),
        &deps.db_pool,
    )
    .await?;

    Ok(post.id)
}

/// Approve a post (make it active)
/// Returns the approved PostId.
pub async fn approve_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<PostId> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Approving post");

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    post_operations::update_post_status(post_id, "active".to_string(), &deps.db_pool).await?;

    Ok(post_id)
}

/// Reject a post (hide forever)
pub async fn reject_post(
    post_id: Uuid,
    reason: String,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<()> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, reason = %reason, "Rejecting post");

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    post_operations::update_post_status(post_id, "rejected".to_string(), &deps.db_pool).await?;

    Ok(())
}

/// Edit and approve a post (fix AI mistakes or improve user content)
/// Returns the approved PostId.
pub async fn edit_and_approve_post(
    post_id: Uuid,
    input: EditPostInput,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<PostId> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, title = ?input.title, "Editing and approving post");

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    post_operations::update_and_approve_post(
        UpdateAndApprovePost::builder()
            .post_id(post_id)
            .title(input.title)
            .description(input.description)
            .description_markdown(input.description_markdown)
            .summary(input.summary)
            .urgency(input.urgency)
            .location(input.location)
            .build(),
        &deps.db_pool,
    )
    .await?;

    Ok(post_id)
}

/// Delete a post
pub async fn delete_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<()> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Deleting post");

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    post_operations::delete_post(post_id, &deps.db_pool).await?;
    Ok(())
}

/// Expire a post
/// Returns the expired PostId.
pub async fn expire_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<PostId> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Expiring post");

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManagePosts)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    post_operations::expire_post(post_id, &deps.db_pool).await?;
    Ok(post_id)
}

/// Archive a post
/// Returns the archived PostId.
pub async fn archive_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<PostId> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Archiving post");

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManagePosts)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    post_operations::archive_post(post_id, &deps.db_pool).await?;
    Ok(post_id)
}

/// Track post view (analytics - public, no auth)
pub async fn track_post_view(post_id: Uuid, deps: &ServerDeps) -> Result<()> {
    let post_id = PostId::from_uuid(post_id);

    post_operations::increment_post_view(post_id, &deps.db_pool).await?;
    Ok(())
}

/// Track post click (analytics - public, no auth)
pub async fn track_post_click(post_id: Uuid, deps: &ServerDeps) -> Result<()> {
    let post_id = PostId::from_uuid(post_id);

    post_operations::increment_post_click(post_id, &deps.db_pool).await?;
    Ok(())
}

// ============================================================================
// Query Actions (Relay pagination)
// ============================================================================

use crate::common::{build_page_info, Cursor, ValidatedPaginationArgs};
use crate::domains::posts::data::{PostConnection, PostEdge, PostType};
use crate::domains::posts::models::Post;

/// Get paginated posts with cursor-based pagination (Relay spec)
///
/// This is the main query action for listing posts with proper pagination.
/// Returns a PostConnection with edges, pageInfo, and totalCount.
pub async fn get_posts_paginated(
    status: Option<&str>,
    source_type: Option<&str>,
    source_id: Option<uuid::Uuid>,
    agent_id: Option<uuid::Uuid>,
    search: Option<&str>,
    args: &ValidatedPaginationArgs,
    deps: &ServerDeps,
) -> Result<PostConnection> {
    let pool = &deps.db_pool;

    // Fetch posts with cursor pagination
    let (posts, has_more) = Post::find_paginated(status, source_type, source_id, agent_id, search, args, pool).await?;

    // Get total count for the filter
    let total_count = Post::count_by_status(status, source_type, source_id, agent_id, search, pool).await? as i32;

    // Build edges with cursors
    let edges: Vec<PostEdge> = posts
        .into_iter()
        .map(|post| {
            let cursor = Cursor::encode_uuid(post.id.into_uuid());
            PostEdge {
                node: PostType::from(post),
                cursor,
            }
        })
        .collect();

    // Build page info
    let page_info = build_page_info(
        has_more,
        args,
        edges.first().map(|e| e.cursor.clone()),
        edges.last().map(|e| e.cursor.clone()),
    );

    Ok(PostConnection {
        edges,
        page_info,
        total_count,
    })
}
