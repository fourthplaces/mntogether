//! Chat effect - handles container and message operations.
//!
//! This effect is a thin orchestration layer that dispatches events to action functions.
//! Following CLAUDE.md: Effects must be thin orchestration layers, business logic in actions.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use crate::domains::chatrooms::actions;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::posts::effects::ServerDeps;

/// Chat Effect - Handles ChatEvent request events
pub struct ChatEffect;

#[async_trait]
impl Effect<ChatEvent, ServerDeps> for ChatEffect {
    type Event = ChatEvent;

    async fn handle(
        &mut self,
        event: ChatEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ChatEvent> {
        match event {
            // =================================================================
            // Request Events → Dispatch to Actions
            // =================================================================
            ChatEvent::CreateContainerRequested {
                container_type,
                entity_id,
                language,
                requested_by,
                with_agent,
            } => {
                actions::create_container(
                    container_type,
                    entity_id,
                    language,
                    requested_by,
                    with_agent,
                    &ctx,
                )
                .await
            }

            ChatEvent::SendMessageRequested {
                container_id,
                content,
                author_id,
                parent_message_id,
            } => {
                actions::create_message(
                    container_id,
                    "user".to_string(),
                    content,
                    author_id,
                    parent_message_id,
                    &ctx,
                )
                .await
            }

            ChatEvent::CreateMessageRequested {
                container_id,
                role,
                content,
                author_id,
                parent_message_id,
            } => {
                actions::create_message(container_id, role, content, author_id, parent_message_id, &ctx)
                    .await
            }

            ChatEvent::GenerateReplyRequested {
                message_id,
                container_id,
            } => actions::generate_reply(message_id, container_id, &ctx).await,

            ChatEvent::GenerateGreetingRequested {
                container_id,
                agent_config,
            } => actions::generate_greeting(container_id, agent_config, &ctx).await,

            // =================================================================
            // Fact Events → Should not reach effect (return error)
            // =================================================================
            ChatEvent::ContainerCreated { .. }
            | ChatEvent::MessageCreated { .. }
            | ChatEvent::MessageFailed { .. }
            | ChatEvent::ReplyGenerationFailed { .. }
            | ChatEvent::GreetingGenerationFailed { .. } => {
                anyhow::bail!(
                    "Fact events should not be dispatched to effects. \
                     They are outputs from effects, not inputs."
                )
            }
        }
    }
}
