//! ChatEffect - Handles chat workflow cascading
//!
//! This effect watches FACT events and calls handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Cascade flow:
//!   ContainerCreated (with_agent) → handle_generate_greeting → MessageCreated
//!   MessageCreated (user role, container has agent) → handle_generate_reply → MessageCreated

use seesaw_core::{effect, EffectContext};
use tracing::{error, info};

use crate::common::AppState;
use crate::domains::chatrooms::events::ChatEvent;
use crate::kernel::ServerDeps;

use super::handlers;

/// Build the chat effect handler.
///
/// This effect watches FACT events and calls handlers directly for cascading.
/// No *Requested events - the effect IS the cascade controller.
pub fn chat_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ChatEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: ContainerCreated with agent → generate greeting
                // =================================================================
                ChatEvent::ContainerCreated {
                    container,
                    with_agent,
                } => {
                    if let Some(agent_config) = with_agent {
                        match handlers::handle_generate_greeting(
                            container.id,
                            agent_config.clone(),
                            ctx.deps(),
                        )
                        .await
                        {
                            Ok(message) => {
                                info!(message_id = %message.id, "Greeting generated");
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to generate greeting");
                            }
                        }
                    }
                    Ok(()) // Terminal - greeting created internally
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
                        match handlers::handle_generate_reply(
                            message.id,
                            message.container_id,
                            ctx.deps(),
                        )
                        .await
                        {
                            Ok(reply) => {
                                info!(reply_id = %reply.id, "Reply generated");
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to generate reply");
                            }
                        }
                    }
                    Ok(()) // Terminal - reply created internally
                }
            }
        },
    )
}
