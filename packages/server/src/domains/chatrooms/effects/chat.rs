//! Chat effect - handles request events and emits fact events.
//!
//! In seesaw Edge pattern:
//!   Edge.execute() → Request Event → Effect → Fact Event → Reducer → Edge.read()

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use crate::domains::chatrooms::actions;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::state::ChatRequestState;
use crate::kernel::ServerDeps;

/// Chat Effect - Handles request events and emits fact events
pub struct ChatEffect;

#[async_trait]
impl Effect<ChatEvent, ServerDeps, ChatRequestState> for ChatEffect {
    type Event = ChatEvent;

    async fn handle(
        &mut self,
        event: ChatEvent,
        ctx: EffectContext<ServerDeps, ChatRequestState>,
    ) -> Result<Option<ChatEvent>> {
        match event {
            // =================================================================
            // External Request Events (from edges)
            // =================================================================

            ChatEvent::CreateContainerRequested {
                container_type,
                entity_id,
                language,
                requested_by,
                with_agent,
            } => {
                let (container, _) = actions::create_container(
                    container_type,
                    entity_id,
                    language,
                    requested_by,
                    with_agent.clone(),
                    &ctx.deps().db_pool,
                )
                .await?;

                Ok(Some(ChatEvent::ContainerCreated {
                    container,
                    with_agent,
                }))
            }

            ChatEvent::SendMessageRequested {
                container_id,
                content,
                author_id,
                parent_message_id,
            } => {
                let (message, _) = actions::create_message(
                    container_id,
                    "user".to_string(),
                    content,
                    author_id,
                    parent_message_id,
                    &ctx.deps().db_pool,
                )
                .await?;

                Ok(Some(ChatEvent::MessageCreated { message }))
            }

            // =================================================================
            // Internal Chain Events (from internal edges)
            // =================================================================

            ChatEvent::GenerateReplyRequested {
                message_id,
                container_id,
            } => actions::generate_reply(message_id, container_id, &ctx).await.map(Some),

            ChatEvent::GenerateGreetingRequested {
                container_id,
                agent_config,
            } => actions::generate_greeting(container_id, agent_config, &ctx).await.map(Some),

            ChatEvent::CreateMessageRequested {
                container_id,
                role,
                content,
                author_id,
                parent_message_id,
            } => {
                let (message, _) = actions::create_message(
                    container_id,
                    role,
                    content,
                    author_id,
                    parent_message_id,
                    &ctx.deps().db_pool,
                )
                .await?;
                Ok(Some(ChatEvent::MessageCreated { message }))
            }

            // =================================================================
            // Fact Events → Terminal, no follow-up needed
            // =================================================================
            ChatEvent::ContainerCreated { .. }
            | ChatEvent::MessageCreated { .. }
            | ChatEvent::MessageFailed { .. }
            | ChatEvent::ReplyGenerationFailed { .. }
            | ChatEvent::GreetingGenerationFailed { .. } => Ok(None),
        }
    }
}
