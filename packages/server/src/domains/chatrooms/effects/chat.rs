//! ChatEffect - Handles chat workflow cascading
//!
//! This effect watches FACT events and calls handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Cascade flow:
//!   ContainerCreated (with_agent) → handle_generate_greeting → MessageCreated
//!   MessageCreated (user role, container has agent) → handle_generate_reply → MessageCreated

use seesaw_core::effect;
use std::sync::Arc;

use crate::common::AppState;
use crate::domains::chatrooms::events::ChatEvent;
use crate::kernel::ServerDeps;

use super::handlers;

/// Build the chat effect handler.
///
/// This effect watches FACT events and calls handlers directly for cascading.
/// No *Requested events - the effect IS the cascade controller.
pub fn chat_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ChatEvent>().run(|event: Arc<ChatEvent>, ctx| async move {
        match event.as_ref() {
            // =================================================================
            // Cascade: ContainerCreated with agent → generate greeting
            // =================================================================
            ChatEvent::ContainerCreated {
                container,
                with_agent,
            } => {
                if let Some(agent_config) = with_agent {
                    handlers::handle_generate_greeting(container.id, agent_config.clone(), &ctx)
                        .await?;
                }
                Ok(())
            }

            // =================================================================
            // Cascade: MessageCreated (user) in container with agent → generate reply
            // =================================================================
            ChatEvent::MessageCreated { message } => {
                // Only cascade for user messages
                if message.role != "user" {
                    return Ok(());
                }

                // Check if container has an agent
                if let Some(_agent_config) =
                    handlers::get_container_agent_config(message.container_id, &ctx.deps().db_pool)
                        .await
                {
                    handlers::handle_generate_reply(message.id, message.container_id, &ctx).await?;
                }
                Ok(())
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            ChatEvent::MessageFailed { .. }
            | ChatEvent::ReplyGenerationFailed { .. }
            | ChatEvent::GreetingGenerationFailed { .. } => Ok(()),
        }
    })
}
