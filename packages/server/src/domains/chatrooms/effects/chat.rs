//! ChatEffect - Handles chat workflow cascading
//!
//! Event-driven pipeline:
//!   ContainerCreated (with_agent) → generate_greeting → MessageCreated
//!   MessageCreated (user role, container has agent) → generate_reply → MessageCreated

use seesaw_core::{effect, EffectContext};
use tracing::info;

use crate::common::AppState;
use crate::domains::chatrooms::actions;
use crate::domains::chatrooms::events::ChatEvent;
use crate::kernel::ServerDeps;

/// Build the chat effect handler.
///
/// Each match arm calls an action directly - no handler indirection.
/// Errors propagate to global on_error() handler.
pub fn chat_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ChatEvent>().id("chat_handler").then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // ContainerCreated with agent → generate greeting
                // =================================================================
                ChatEvent::ContainerCreated {
                    container,
                    with_agent,
                } => {
                    if let Some(agent_config) = with_agent {
                        let message =
                            actions::generate_greeting(container.id, agent_config, ctx.deps())
                                .await?;
                        info!(message_id = %message.id, "Greeting generated");
                    }
                    Ok(())
                }

                // =================================================================
                // MessageCreated (user) in container with agent → generate reply
                // =================================================================
                ChatEvent::MessageCreated { message } => {
                    // Only cascade for user messages
                    if message.role != "user" {
                        return Ok(());
                    }

                    // Check if container has an agent
                    if actions::get_container_agent_config(message.container_id, &ctx.deps().db_pool)
                        .await
                        .is_some()
                    {
                        let reply =
                            actions::generate_reply(message.id, message.container_id, ctx.deps())
                                .await?;
                        info!(reply_id = %reply.id, "Reply generated");
                    }
                    Ok(())
                }
            }
        },
    )
}
