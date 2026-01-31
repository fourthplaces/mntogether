//! Chat effect - handles container and message creation.
//!
//! This effect is a thin orchestration layer that dispatches commands to handler functions.
//! Following CLAUDE.md: Effects must be thin orchestration layers, business logic in handlers.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::info;

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::commands::ChatCommand;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::listings::effects::deps::ServerDeps;
use crate::domains::tag::{Tag, Taggable};

/// Chat Effect - Handles CreateContainer and CreateMessage commands
pub struct ChatEffect;

#[async_trait]
impl Effect<ChatCommand, ServerDeps> for ChatEffect {
    type Event = ChatEvent;

    async fn execute(
        &self,
        cmd: ChatCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ChatEvent> {
        match cmd {
            ChatCommand::CreateContainer {
                container_type,
                entity_id,
                language,
                requested_by: _,
                with_agent,
            } => handle_create_container(container_type, entity_id, language, with_agent, &ctx).await,

            ChatCommand::CreateMessage {
                container_id,
                role,
                content,
                author_id,
                parent_message_id,
            } => {
                handle_create_message(container_id, role, content, author_id, parent_message_id, &ctx)
                    .await
            }
        }
    }
}

// =============================================================================
// Handler Functions (Business Logic)
// =============================================================================

async fn handle_create_container(
    container_type: String,
    entity_id: Option<uuid::Uuid>,
    language: String,
    with_agent: Option<String>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatEvent> {
    info!(container_type = %container_type, ?with_agent, "Creating chat container");

    let container = Container::create(
        container_type.clone(),
        entity_id,
        language,
        &ctx.deps().db_pool,
    )
    .await?;

    // Tag container with agent config if provided
    if let Some(ref agent_config) = with_agent {
        info!(container_id = %container.id, agent_config = %agent_config, "Tagging container with agent");
        let tag = Tag::find_or_create(
            "with_agent",
            agent_config,
            None,
            &ctx.deps().db_pool,
        )
        .await?;
        Taggable::create_container_tag(container.id, tag.id, &ctx.deps().db_pool).await?;
    }

    Ok(ChatEvent::ContainerCreated {
        container_id: container.id,
        container_type,
        with_agent,
    })
}

async fn handle_create_message(
    container_id: ContainerId,
    role: String,
    content: String,
    author_id: Option<MemberId>,
    parent_message_id: Option<MessageId>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatEvent> {
    info!(container_id = %container_id, role = %role, "Creating message");

    // Get next sequence number
    let sequence_number = Message::next_sequence_number(container_id, &ctx.deps().db_pool).await?;

    // Create message
    let message = Message::create(
        container_id,
        role.clone(),
        content.clone(),
        author_id,
        Some("approved".to_string()), // AI chat messages auto-approved
        parent_message_id,
        sequence_number,
        &ctx.deps().db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &ctx.deps().db_pool).await?;

    Ok(ChatEvent::MessageCreated {
        message_id: message.id,
        container_id,
        role,
        content,
        author_id,
    })
}
