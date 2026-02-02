//! Chat domain events.
//!
//! Events are immutable facts about what happened. They follow the seesaw pattern:
//! - Request events: User intent (from edges)
//! - Fact events: What actually happened (from effects)
//!
//! Architecture:
//!   Edge.execute() → Request Event → Effect → Fact Event → Reducer → Edge.read()

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{ContainerId, IntoNatsPayload, MemberId, MessageId};
use crate::domains::chatrooms::models::{Container, Message};

/// Chat domain events - immutable facts
#[derive(Debug, Clone)]
pub enum ChatEvent {
    // =========================================================================
    // Request Events (from edges and internal edges)
    // =========================================================================
    /// User requests to create a new chat container
    CreateContainerRequested {
        container_type: String,
        entity_id: Option<uuid::Uuid>,
        language: String,
        requested_by: Option<MemberId>,
        /// Optional agent config - when set, tags container with with_agent tag
        with_agent: Option<String>,
    },

    /// User sends a message
    SendMessageRequested {
        container_id: ContainerId,
        content: String,
        author_id: Option<MemberId>,
        parent_message_id: Option<MessageId>,
    },

    /// Request to create a message with specific role (used by internal edges)
    CreateMessageRequested {
        container_id: ContainerId,
        role: String,
        content: String,
        author_id: Option<MemberId>,
        parent_message_id: Option<MessageId>,
    },

    /// Request to generate an AI reply to a message (triggered by internal edge)
    GenerateReplyRequested {
        message_id: MessageId,
        container_id: ContainerId,
    },

    /// Request to generate an AI greeting for a new container (triggered by internal edge)
    GenerateGreetingRequested {
        container_id: ContainerId,
        agent_config: String,
    },

    // =========================================================================
    // Fact Events (from effects - what actually happened)
    // =========================================================================
    /// Container was created
    ContainerCreated {
        container: Container,
        /// Agent config if this container has an agent enabled
        with_agent: Option<String>,
    },

    /// Message was created
    MessageCreated {
        message: Message,
    },

    /// Message creation failed
    MessageFailed { container_id: ContainerId, reason: String },

    /// AI reply generation failed
    ReplyGenerationFailed {
        message_id: MessageId,
        container_id: ContainerId,
        reason: String,
    },

    /// AI greeting generation failed
    GreetingGenerationFailed {
        container_id: ContainerId,
        reason: String,
    },
}

/// Agent messaging events (pure effect pattern).
///
/// These events represent facts about AI-generated content.
/// A machine observes these and emits ChatCommand::CreateMessage.
///
/// Causality chain:
/// ```text
/// GenerateChatReplyCommand
///     → GenerateChatReplyEffect
///     → ChatMessagingEvent::ReplyGenerated
///     → AgentMessagingMachine
///     → ChatCommand::CreateMessage
///     → ChatEffect
///     → ChatEvent::MessageCreated
/// ```
#[derive(Debug, Clone)]
pub enum ChatMessagingEvent {
    /// Agent reply text was generated (fact about AI output).
    /// A machine will observe this and emit ChatCommand::CreateMessage.
    ReplyGenerated {
        container_id: ContainerId,
        response_to_id: MessageId,
        author_id: MemberId, // Agent's member ID
        text: String,
    },

    /// Reply generation was skipped (no agent, author is agent, etc.)
    Skipped { reason: &'static str },
}

/// Typing event (ephemeral signal - not persisted).
///
/// Used for real-time typing indicators via NATS.
#[derive(Debug, Clone)]
pub enum TypingEvent {
    /// Someone started typing in a container
    Started {
        container_id: ContainerId,
        member_id: MemberId,
    },
}

// =============================================================================
// NATS Publishing
// =============================================================================

/// Serializable format for chat events published to NATS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEventPayload {
    /// Message was created
    MessageCreated {
        message_id: String,
        container_id: String,
        role: String,
        content: String,
        author_id: Option<String>,
    },
    /// Container was created
    ContainerCreated {
        container_id: String,
        container_type: String,
        with_agent: Option<String>,
    },
}

impl IntoNatsPayload for ChatEvent {
    fn container_id(&self) -> Option<Uuid> {
        match self {
            // Request events are not published
            ChatEvent::CreateContainerRequested { .. }
            | ChatEvent::SendMessageRequested { .. }
            | ChatEvent::CreateMessageRequested { .. }
            | ChatEvent::GenerateReplyRequested { .. }
            | ChatEvent::GenerateGreetingRequested { .. } => None,

            // Fact events with container_id are published
            ChatEvent::ContainerCreated { container, .. } => Some(container.id.into()),
            ChatEvent::MessageCreated { message } => Some(message.container_id.into()),
            ChatEvent::MessageFailed { container_id, .. } => Some((*container_id).into()),
            ChatEvent::ReplyGenerationFailed { container_id, .. } => Some((*container_id).into()),
            ChatEvent::GreetingGenerationFailed { container_id, .. } => Some((*container_id).into()),
        }
    }

    fn into_payload(&self) -> serde_json::Value {
        match self {
            ChatEvent::MessageCreated { message } => {
                serde_json::to_value(ChatEventPayload::MessageCreated {
                    message_id: message.id.to_string(),
                    container_id: message.container_id.to_string(),
                    role: message.role.clone(),
                    content: message.content.clone(),
                    author_id: message.author_id.map(|id| id.to_string()),
                })
                .unwrap_or_default()
            }

            ChatEvent::ContainerCreated { container, with_agent } => {
                serde_json::to_value(ChatEventPayload::ContainerCreated {
                    container_id: container.id.to_string(),
                    container_type: container.container_type.clone(),
                    with_agent: with_agent.clone(),
                })
                .unwrap_or_default()
            }

            // Other events don't have payload representations
            _ => serde_json::Value::Null,
        }
    }

    fn subject_suffix() -> &'static str {
        "messages"
    }
}

/// Serializable format for typing events published to NATS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingEventPayload {
    pub container_id: String,
    pub member_id: String,
}

impl IntoNatsPayload for TypingEvent {
    fn container_id(&self) -> Option<Uuid> {
        match self {
            TypingEvent::Started { container_id, .. } => Some((*container_id).into()),
        }
    }

    fn into_payload(&self) -> serde_json::Value {
        match self {
            TypingEvent::Started {
                container_id,
                member_id,
            } => serde_json::to_value(TypingEventPayload {
                container_id: container_id.to_string(),
                member_id: member_id.to_string(),
            })
            .unwrap_or_default(),
        }
    }

    fn subject_suffix() -> &'static str {
        "typing"
    }

    fn exclude_member_id(&self) -> Option<Uuid> {
        // Exclude the sender from typing notifications
        match self {
            TypingEvent::Started { member_id, .. } => Some((*member_id).into()),
        }
    }
}
