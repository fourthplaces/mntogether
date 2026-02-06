//! AgentEffect - Generates AI responses for chat messages.
//!
//! Watches ChatEvent facts from the chatrooms domain:
//!   ContainerCreated (with_agent) → generate_greeting
//!   MessageCreated (user role, container has agent) → generate_reply_streaming
//!     Falls back to generate_reply (blocking) if streaming fails to start.

use std::sync::Arc;
use std::time::Duration;

use seesaw_core::{effect, EffectContext};
use tracing::{info, warn};

use crate::common::AppState;
use crate::domains::agents::actions;
use crate::domains::chatrooms::events::ChatEvent;
use crate::kernel::ServerDeps;

/// Build the agent effect handler.
///
/// Cross-domain effect: watches ChatEvent (chatrooms) → calls agent actions.
/// Queued with retry and timeout since it calls OpenAI API.
pub fn agent_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ChatEvent>()
        .id("agent_handler")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(120))
        .then(
            |event: Arc<ChatEvent>, ctx: EffectContext<AppState, ServerDeps>| async move {
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
                            info!(message_id = %message.id, "Agent greeting generated");
                        }
                        Ok(())
                    }

                    // =================================================================
                    // MessageCreated (user) in container with agent → stream reply
                    // =================================================================
                    ChatEvent::MessageCreated { message } => {
                        if message.role != "user" {
                            return Ok(());
                        }

                        if actions::get_container_agent_config(
                            message.container_id,
                            &ctx.deps().db_pool,
                        )
                        .await
                        .is_some()
                        {
                            // Try streaming first, fall back to blocking
                            match actions::generate_reply_streaming(
                                message.id,
                                message.container_id,
                                ctx.deps(),
                            )
                            .await
                            {
                                Ok(reply) => {
                                    info!(reply_id = %reply.id, "Streaming agent reply generated");
                                }
                                Err(e) => {
                                    warn!(error = %e, "Streaming reply failed, falling back to blocking");
                                    let reply = actions::generate_reply(
                                        message.id,
                                        message.container_id,
                                        ctx.deps(),
                                    )
                                    .await?;
                                    info!(reply_id = %reply.id, "Blocking agent reply generated (fallback)");
                                }
                            }
                        }
                        Ok(())
                    }
                }
            },
        )
}
