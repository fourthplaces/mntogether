use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Need Effect - Handles CreateNeed, UpdateNeedStatus, UpdateNeedAndApprove, CreatePost, GenerateNeedEmbedding commands
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
                volunteer_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
                submission_type,
            } => {
                let ip_str = ip_address.map(|ip| ip.to_string());

                let need = super::need_operations::create_need(
                    volunteer_id,
                    organization_name.clone(),
                    title,
                    description,
                    contact_info,
                    urgency,
                    location,
                    ip_str,
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

            OrganizationCommand::UpdateNeedStatus {
                need_id,
                status,
                rejection_reason,
            } => {
                let updated_status = super::need_operations::update_need_status(
                    need_id,
                    status.clone(),
                    &ctx.deps().db_pool,
                )
                .await?;

                if updated_status == "active" {
                    Ok(OrganizationEvent::NeedApproved { need_id })
                } else if updated_status == "rejected" {
                    Ok(OrganizationEvent::NeedRejected {
                        need_id,
                        reason: rejection_reason
                            .unwrap_or_else(|| "No reason provided".to_string()),
                    })
                } else {
                    Ok(OrganizationEvent::NeedUpdated { need_id })
                }
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
            } => {
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

            OrganizationCommand::CreatePost {
                need_id,
                created_by,
                custom_title,
                custom_description,
                expires_in_days,
            } => {
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

            OrganizationCommand::GenerateNeedEmbedding { need_id } => {
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

            _ => anyhow::bail!("NeedEffect: Unexpected command"),
        }
    }
}
