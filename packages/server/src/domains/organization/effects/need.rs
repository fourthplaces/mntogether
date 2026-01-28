use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use serde_json::Value as JsonValue;

use super::deps::ServerDeps;
use crate::common::{MemberId, NeedId, PostId};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Need Effect - Handles CreateNeed, UpdateNeedStatus, UpdateNeedAndApprove, CreatePost, GenerateNeedEmbedding commands
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct NeedEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for NeedEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::CreateNeed {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
                submission_type,
            } => {
                handle_create_need(
                    member_id,
                    organization_name,
                    title,
                    description,
                    contact_info,
                    urgency,
                    location,
                    ip_address,
                    submission_type,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::UpdateNeedStatus {
                need_id,
                status,
                rejection_reason,
                requested_by,
                is_admin,
            } => {
                handle_update_need_status(
                    need_id,
                    status,
                    rejection_reason,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::UpdateNeedAndApprove {
                need_id,
                title,
                description,
                description_markdown,
                tldr,
                contact_info,
                urgency,
                location,
                requested_by,
                is_admin,
            } => {
                handle_update_need_and_approve(
                    need_id,
                    title,
                    description,
                    description_markdown,
                    tldr,
                    contact_info,
                    urgency,
                    location,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::CreatePost {
                need_id,
                created_by,
                custom_title,
                custom_description,
                expires_in_days,
            } => {
                handle_create_post(
                    need_id,
                    created_by,
                    custom_title,
                    custom_description,
                    expires_in_days,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::GenerateNeedEmbedding { need_id } => {
                handle_generate_need_embedding(need_id, &ctx).await
            }

            OrganizationCommand::CreateCustomPost {
                need_id,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                expires_in_days,
                created_by,
                requested_by,
                is_admin,
            } => {
                handle_create_custom_post(
                    need_id,
                    custom_title,
                    custom_description,
                    custom_tldr,
                    targeting_hints,
                    expires_in_days,
                    created_by,
                    requested_by,
                    is_admin,
                    &ctx,
                )
                .await
            }

            OrganizationCommand::RepostNeed {
                need_id,
                created_by,
                requested_by,
                is_admin,
            } => {
                handle_repost_need(need_id, created_by, requested_by, is_admin, &ctx)
                    .await
            }

            OrganizationCommand::ExpirePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_expire_post(post_id, requested_by, is_admin, &ctx).await,

            OrganizationCommand::ArchivePost {
                post_id,
                requested_by,
                is_admin,
            } => handle_archive_post(post_id, requested_by, is_admin, &ctx).await,

            OrganizationCommand::IncrementPostView { post_id } => {
                handle_increment_post_view(post_id, &ctx).await
            }

            OrganizationCommand::IncrementPostClick { post_id } => {
                handle_increment_post_click(post_id, &ctx).await
            }

            _ => anyhow::bail!("NeedEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Need handlers
// ============================================================================

async fn handle_create_need(
    member_id: Option<MemberId>,
    organization_name: String,
    title: String,
    description: String,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    ip_address: Option<String>,
    submission_type: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    let need = super::need_operations::create_need(
        member_id,
        organization_name.clone(),
        title,
        description,
        contact_info,
        urgency,
        location,
        ip_address,
        submission_type.clone(),
        None, // source_id
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::NeedCreated {
        need_id: need.id,
        organization_name: need.organization_name,
        title: need.title,
        submission_type,
    })
}

async fn handle_update_need_status(
    need_id: NeedId,
    status: String,
    rejection_reason: Option<String>,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can update need status
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdateNeedStatus".to_string(),
            reason: "Only administrators can approve or reject needs".to_string(),
        });
    }

    let updated_status =
        super::need_operations::update_need_status(need_id, status.clone(), &ctx.deps().db_pool)
            .await?;

    if updated_status == "active" {
        Ok(OrganizationEvent::NeedApproved { need_id })
    } else if updated_status == "rejected" {
        Ok(OrganizationEvent::NeedRejected {
            need_id,
            reason: rejection_reason.unwrap_or_else(|| "No reason provided".to_string()),
        })
    } else {
        Ok(OrganizationEvent::NeedUpdated { need_id })
    }
}

async fn handle_update_need_and_approve(
    need_id: NeedId,
    title: Option<String>,
    description: Option<String>,
    description_markdown: Option<String>,
    tldr: Option<String>,
    contact_info: Option<JsonValue>,
    urgency: Option<String>,
    location: Option<String>,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can edit and approve needs
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "UpdateNeedAndApprove".to_string(),
            reason: "Only administrators can edit and approve needs".to_string(),
        });
    }

    super::need_operations::update_and_approve_need(
        need_id,
        title,
        description,
        description_markdown,
        tldr,
        contact_info,
        urgency,
        location,
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::NeedApproved { need_id })
}

async fn handle_generate_need_embedding(
    need_id: NeedId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    match super::need_operations::generate_need_embedding(
        need_id,
        ctx.deps().embedding_service.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(dimensions) => Ok(OrganizationEvent::NeedEmbeddingGenerated {
            need_id,
            dimensions,
        }),
        Err(e) => Ok(OrganizationEvent::NeedEmbeddingFailed {
            need_id,
            reason: e.to_string(),
        }),
    }
}

// ============================================================================
// Post handlers
// ============================================================================

async fn handle_create_post(
    need_id: NeedId,
    created_by: Option<MemberId>,
    custom_title: Option<String>,
    custom_description: Option<String>,
    expires_in_days: Option<i64>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    let post = super::need_operations::create_post_for_need(
        need_id,
        created_by,
        custom_title,
        custom_description,
        expires_in_days,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::PostCreated {
        post_id: post.id,
        need_id,
    })
}

async fn handle_create_custom_post(
    need_id: NeedId,
    custom_title: Option<String>,
    custom_description: Option<String>,
    custom_tldr: Option<String>,
    targeting_hints: Option<JsonValue>,
    expires_in_days: Option<i64>,
    created_by: MemberId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can create custom posts
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "CreateCustomPost".to_string(),
            reason: "Only administrators can create custom posts".to_string(),
        });
    }

    let post = super::need_operations::create_custom_post(
        need_id,
        Some(created_by),
        custom_title,
        custom_description,
        custom_tldr,
        targeting_hints,
        expires_in_days,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::PostCreated {
        post_id: post.id,
        need_id,
    })
}

async fn handle_repost_need(
    need_id: NeedId,
    created_by: MemberId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can repost needs
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "RepostNeed".to_string(),
            reason: "Only administrators can repost needs".to_string(),
        });
    }

    let post = super::need_operations::create_post_for_need(
        need_id,
        Some(created_by),
        None,
        None,
        None,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(OrganizationEvent::PostCreated {
        post_id: post.id,
        need_id,
    })
}

async fn handle_expire_post(
    post_id: PostId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can expire posts
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ExpirePost".to_string(),
            reason: "Only administrators can expire posts".to_string(),
        });
    }

    super::need_operations::expire_post(post_id, &ctx.deps().db_pool).await?;

    Ok(OrganizationEvent::PostExpired { post_id })
}

async fn handle_archive_post(
    post_id: PostId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    // Authorization check - only admins can archive posts
    if !is_admin {
        return Ok(OrganizationEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ArchivePost".to_string(),
            reason: "Only administrators can archive posts".to_string(),
        });
    }

    super::need_operations::archive_post(post_id, &ctx.deps().db_pool).await?;

    Ok(OrganizationEvent::PostArchived { post_id })
}

async fn handle_increment_post_view(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    super::need_operations::increment_post_view(post_id, &ctx.deps().db_pool).await?;
    Ok(OrganizationEvent::PostViewed { post_id })
}

async fn handle_increment_post_click(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    super::need_operations::increment_post_click(post_id, &ctx.deps().db_pool).await?;
    Ok(OrganizationEvent::PostClicked { post_id })
}
