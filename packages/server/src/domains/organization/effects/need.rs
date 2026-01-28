use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::utils::generate_content_hash;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::{need::OrganizationNeed, post::Post, NeedStatus};

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
                // Generate content hash for deduplication
                let content_hash = generate_content_hash(&format!(
                    "{} {} {}",
                    title, description, organization_name
                ));

                // Generate TLDR (first 100 chars of description)
                let tldr = if description.len() > 100 {
                    format!("{}...", &description[..97])
                } else {
                    description.clone()
                };

                // Convert IP address to string
                let ip_str = ip_address.map(|ip| ip.to_string());

                // Create need using model method
                let need = sqlx::query_as!(
                    OrganizationNeed,
                    r#"
                    INSERT INTO organization_needs (
                        organization_name,
                        title,
                        description,
                        tldr,
                        contact_info,
                        urgency,
                        location,
                        status,
                        content_hash,
                        submission_type,
                        submitted_by_volunteer_id,
                        submitted_from_ip
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::inet)
                    RETURNING
                        id,
                        organization_name,
                        title,
                        description,
                        description_markdown,
                        tldr,
                        contact_info,
                        urgency,
                        status as "status!: String",
                        content_hash,
                        source_id,
                        submission_type,
                        submitted_by_volunteer_id,
                        location,
                        last_seen_at,
                        disappeared_at,
                        created_at,
                        updated_at
                    "#,
                    organization_name,
                    title,
                    description,
                    tldr,
                    contact_info,
                    urgency,
                    location,
                    NeedStatus::PendingApproval.to_string(),
                    content_hash,
                    submission_type,
                    volunteer_id,
                    ip_str
                )
                .fetch_one(&ctx.deps().db_pool)
                .await
                .context("Failed to create need")?;

                // Return fact event
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
                // Update status using model method
                OrganizationNeed::update_status(need_id, &status, &ctx.deps().db_pool)
                    .await
                    .context("Failed to update need status")?;

                // Return appropriate fact event
                if status == "active" {
                    Ok(OrganizationEvent::NeedApproved { need_id })
                } else if status == "rejected" {
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
                // Update need content and approve
                OrganizationNeed::update_content(
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
                .await
                .context("Failed to update need content")?;

                // Set status to active
                OrganizationNeed::update_status(need_id, "active", &ctx.deps().db_pool)
                    .await
                    .context("Failed to approve need")?;

                // Return fact event
                Ok(OrganizationEvent::NeedApproved { need_id })
            }

            OrganizationCommand::CreatePost {
                need_id,
                created_by,
                custom_title,
                custom_description,
                expires_in_days,
            } => {
                // Create and publish post
                let post = if custom_title.is_some() || custom_description.is_some() {
                    Post::create_and_publish_custom(
                        need_id,
                        created_by,
                        custom_title,
                        custom_description,
                        None, // custom_tldr
                        None, // targeting_hints
                        expires_in_days,
                        &ctx.deps().db_pool,
                    )
                    .await
                    .context("Failed to create post")?
                } else {
                    Post::create_and_publish(need_id, created_by, expires_in_days, &ctx.deps().db_pool)
                        .await
                        .context("Failed to create post")?
                };

                // Return fact event
                Ok(OrganizationEvent::PostCreated {
                    post_id: post.id,
                    need_id,
                })
            }

            OrganizationCommand::GenerateNeedEmbedding { need_id } => {
                // Get need from database
                let need = OrganizationNeed::find_by_id(need_id, &ctx.deps().db_pool)
                    .await
                    .context("Failed to find need")?;

                // Generate embedding from description
                let embedding = match ctx.deps().embedding_service.generate(&need.description).await {
                    Ok(emb) => emb,
                    Err(e) => {
                        return Ok(OrganizationEvent::NeedEmbeddingFailed {
                            need_id,
                            reason: format!("Embedding generation failed: {}", e),
                        });
                    }
                };

                // Update need with embedding
                if let Err(e) = OrganizationNeed::update_embedding(need_id, &embedding, &ctx.deps().db_pool).await {
                    return Ok(OrganizationEvent::NeedEmbeddingFailed {
                        need_id,
                        reason: format!("Failed to save embedding: {}", e),
                    });
                }

                Ok(OrganizationEvent::NeedEmbeddingGenerated {
                    need_id,
                    dimensions: embedding.len(),
                })
            }

            _ => anyhow::bail!("NeedEffect: Unexpected command"),
        }
    }
}
