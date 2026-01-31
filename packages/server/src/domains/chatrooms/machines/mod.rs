//! Chat domain machines.
//!
//! Machines are pure decision makers - they convert events to commands.
//! NO IO, NO async, just synchronous state transitions.
//!
//! # Architecture
//!
//! ```text
//! ChatEvent::SendMessageRequested
//!     → ChatEventMachine
//!     → ChatCommand::CreateMessage
//!     → ChatEffect
//!     → ChatEvent::MessageCreated
//!     → AgentReplyMachine
//!     → GenerateChatReplyCommand
//!     → GenerateChatReplyEffect
//!     → ChatMessagingEvent::ReplyGenerated
//!     → AgentMessagingMachine
//!     → ChatCommand::CreateMessage
//!     → ChatEffect
//!     → ChatEvent::MessageCreated
//! ```

use crate::domains::chatrooms::commands::{ChatCommand, GenerateAgentGreetingCommand, GenerateChatReplyCommand};
use crate::domains::chatrooms::events::{ChatEvent, ChatMessagingEvent};

// =============================================================================
// ChatEventMachine - Routes request events to commands
// =============================================================================

/// Machine that routes chat request events to commands.
///
/// This is the entry point for the chat domain. It converts user intents
/// (request events from edges) into commands for effects to execute.
pub struct ChatEventMachine;

impl Default for ChatEventMachine {
    fn default() -> Self {
        Self
    }
}

impl seesaw_core::Machine for ChatEventMachine {
    type Event = ChatEvent;
    type Command = ChatCommand;

    fn decide(&mut self, event: &ChatEvent) -> Option<ChatCommand> {
        match event {
            // =========================================================================
            // Request Events → Commands
            // =========================================================================
            ChatEvent::CreateContainerRequested {
                container_type,
                entity_id,
                language,
                requested_by,
                with_agent,
            } => Some(ChatCommand::CreateContainer {
                container_type: container_type.clone(),
                entity_id: *entity_id,
                language: language.clone(),
                requested_by: *requested_by,
                with_agent: with_agent.clone(),
            }),

            ChatEvent::SendMessageRequested {
                container_id,
                content,
                author_id,
                parent_message_id,
            } => Some(ChatCommand::CreateMessage {
                container_id: *container_id,
                role: "user".to_string(),
                content: content.clone(),
                author_id: *author_id,
                parent_message_id: *parent_message_id,
            }),

            // =========================================================================
            // Fact Events - no commands from this machine
            // =========================================================================
            ChatEvent::ContainerCreated { .. }
            | ChatEvent::MessageCreated { .. }
            | ChatEvent::MessageFailed { .. } => None,
        }
    }
}

// =============================================================================
// AgentReplyMachine - Schedules agent replies for user messages
// =============================================================================

/// Machine that schedules agent reply generation for new user messages.
///
/// When a user message is created, this machine emits a GenerateChatReplyCommand
/// which triggers the AI to generate a response.
///
/// Only triggers for "user" role messages to prevent loops.
pub struct AgentReplyMachine;

impl Default for AgentReplyMachine {
    fn default() -> Self {
        Self
    }
}

impl seesaw_core::Machine for AgentReplyMachine {
    type Event = ChatEvent;
    type Command = GenerateChatReplyCommand;

    fn decide(&mut self, event: &ChatEvent) -> Option<GenerateChatReplyCommand> {
        match event {
            ChatEvent::MessageCreated {
                message_id,
                container_id,
                role,
                ..
            } => {
                // Only trigger for user messages, not assistant messages
                if role == "user" {
                    Some(GenerateChatReplyCommand::new(*message_id, *container_id))
                } else {
                    None
                }
            }

            // Other events don't trigger agent replies
            ChatEvent::CreateContainerRequested { .. }
            | ChatEvent::SendMessageRequested { .. }
            | ChatEvent::ContainerCreated { .. }
            | ChatEvent::MessageFailed { .. } => None,
        }
    }
}

// =============================================================================
// AgentMessagingMachine - Converts generated text to message commands
// =============================================================================

/// Machine that converts agent messaging events to message commands.
///
/// This implements the pure effect pattern:
/// - `ChatMessagingEvent::ReplyGenerated` → `ChatCommand::CreateMessage`
///
/// The agent messaging effect generates text and returns a fact.
/// This machine orchestrates by deciding to create a message.
/// Message creation happens in ChatEffect, which emits ChatEvent::MessageCreated.
pub struct AgentMessagingMachine;

impl Default for AgentMessagingMachine {
    fn default() -> Self {
        Self
    }
}

impl seesaw_core::Machine for AgentMessagingMachine {
    type Event = ChatMessagingEvent;
    type Command = ChatCommand;

    fn decide(&mut self, event: &ChatMessagingEvent) -> Option<ChatCommand> {
        match event {
            ChatMessagingEvent::ReplyGenerated {
                container_id,
                response_to_id,
                author_id,
                text,
            } => Some(ChatCommand::CreateMessage {
                container_id: *container_id,
                role: "assistant".to_string(),
                content: text.clone(),
                author_id: Some(*author_id),
                parent_message_id: Some(*response_to_id),
            }),

            ChatMessagingEvent::Skipped { .. } => None,
        }
    }
}

// =============================================================================
// AgentGreetingMachine - Generates greeting when container has agent
// =============================================================================

/// Machine that generates an agent greeting when a container is created with an agent.
///
/// When a container is created with the `with_agent` tag, this machine emits
/// a `GenerateAgentGreetingCommand` which triggers the AI to generate a greeting.
pub struct AgentGreetingMachine;

impl Default for AgentGreetingMachine {
    fn default() -> Self {
        Self
    }
}

impl seesaw_core::Machine for AgentGreetingMachine {
    type Event = ChatEvent;
    type Command = GenerateAgentGreetingCommand;

    fn decide(&mut self, event: &ChatEvent) -> Option<GenerateAgentGreetingCommand> {
        match event {
            ChatEvent::ContainerCreated {
                container_id,
                with_agent: Some(agent_config),
                ..
            } => {
                // Generate greeting when container has an agent
                Some(GenerateAgentGreetingCommand::new(*container_id, agent_config.clone()))
            }

            // Other events don't trigger greetings
            _ => None,
        }
    }
}
