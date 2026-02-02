//! Post CRUD actions - entry-point functions for post operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return final models.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{AppState, MemberId, PostId};
use crate::domains::posts::data::{EditPostInput, SubmitPostInput};
use crate::domains::posts::effects::post_operations;
use crate::domains::posts::events::PostEvent;
use crate::domains::posts::models::Post;
use crate::kernel::ServerDeps;

/// Submit a post from user input (public, goes to pending_approval)
/// Returns the created Post directly.
pub async fn submit_post(
    input: SubmitPostInput,
    member_id: Option<Uuid>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    info!(org = %input.organization_name, title = %input.title, member_id = ?member_id, "Submitting user post");

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());
    let member_id_typed = member_id.map(MemberId::from_uuid);

    let post = post_operations::create_post(
        member_id_typed,
        input.organization_name.clone(),
        input.title,
        input.description,
        contact_json,
        input.urgency,
        input.location,
        None, // ip_address
        "user_submitted".to_string(),
        None, // domain_id
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    ctx.emit(PostEvent::PostEntryCreated {
        post_id: post.id,
        organization_name: post.organization_name.clone(),
        title: post.title.clone(),
        submission_type: "user_submitted".to_string(),
    });

    Ok(post)
}

/// Approve a post (make it active)
/// Returns the updated Post directly.
pub async fn approve_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Approving post");

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ApprovePost".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    post_operations::update_post_status(post_id, "active".to_string(), &ctx.deps().db_pool).await?;

    ctx.emit(PostEvent::PostApproved { post_id });

    Post::find_by_id(post_id, &ctx.deps().db_pool)
        .await?
        .context("Post not found after approval")
}

/// Reject a post (hide forever)
/// Returns true on success.
pub async fn reject_post(
    post_id: Uuid,
    reason: String,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, reason = %reason, "Rejecting post");

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RejectPost".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    post_operations::update_post_status(post_id, "rejected".to_string(), &ctx.deps().db_pool)
        .await?;

    ctx.emit(PostEvent::PostRejected {
        post_id,
        reason: reason.clone(),
    });

    Ok(true)
}

/// Edit and approve a post (fix AI mistakes or improve user content)
/// Returns the updated Post directly.
pub async fn edit_and_approve_post(
    post_id: Uuid,
    input: EditPostInput,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, title = ?input.title, "Editing and approving post");

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManageNeeds)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "EditAndApprovePost".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    post_operations::update_and_approve_post(
        post_id,
        input.title,
        input.description,
        input.description_markdown,
        input.tldr,
        None, // contact_info
        input.urgency,
        input.location,
        &ctx.deps().db_pool,
    )
    .await?;

    ctx.emit(PostEvent::PostApproved { post_id });

    Post::find_by_id(post_id, &ctx.deps().db_pool)
        .await?
        .context("Post not found after edit")
}

/// Delete a post
/// Returns true on success.
pub async fn delete_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Deleting post");

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "DeletePost".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    post_operations::delete_post(post_id, &ctx.deps().db_pool).await?;
    ctx.emit(PostEvent::PostDeleted { post_id });
    Ok(true)
}

/// Expire a post
/// Returns the updated Post directly.
pub async fn expire_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Expiring post");

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ExpirePost".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    post_operations::expire_post(post_id, &ctx.deps().db_pool).await?;
    ctx.emit(PostEvent::PostExpired { post_id });

    Post::find_by_id(post_id, &ctx.deps().db_pool)
        .await?
        .context("Post not found after expiring")
}

/// Archive a post
/// Returns the updated Post directly.
pub async fn archive_post(
    post_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    let post_id = PostId::from_uuid(post_id);
    let requested_by = MemberId::from_uuid(member_id);

    info!(post_id = %post_id, "Archiving post");

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::ManagePosts)
        .check(ctx.deps())
        .await
    {
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ArchivePost".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    post_operations::archive_post(post_id, &ctx.deps().db_pool).await?;
    ctx.emit(PostEvent::PostArchived { post_id });

    Post::find_by_id(post_id, &ctx.deps().db_pool)
        .await?
        .context("Post not found after archiving")
}

/// Track post view (analytics - public, no auth)
/// Returns true on success.
pub async fn track_post_view(
    post_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let post_id = PostId::from_uuid(post_id);

    post_operations::increment_post_view(post_id, &ctx.deps().db_pool).await?;
    ctx.emit(PostEvent::PostViewed { post_id });
    Ok(true)
}

/// Track post click (analytics - public, no auth)
/// Returns true on success.
pub async fn track_post_click(
    post_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let post_id = PostId::from_uuid(post_id);

    post_operations::increment_post_click(post_id, &ctx.deps().db_pool).await?;
    ctx.emit(PostEvent::PostClicked { post_id });
    Ok(true)
}
