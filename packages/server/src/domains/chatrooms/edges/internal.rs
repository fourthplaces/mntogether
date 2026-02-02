//! Chat domain internal edges - event-to-event reactions
//!
//! Internal edges observe fact events and emit new request events.
//! This replaces the machine's decide() logic in seesaw 0.3.0.
//!
//! Flow:
//!   Fact Event → Internal Edge → Option<Request Event>
//!
//! The engine calls these edges after effects produce fact events.
//! If an edge returns Some(event), that event is dispatched to effects.

use crate::domains::chatrooms::events::ChatEvent;

/// React to MessageCreated by triggering AI reply generation for user messages.
///
/// When a user sends a message (role="user"), we want to generate an AI reply.
/// This edge observes the MessageCreated fact and emits a GenerateReplyRequested request.
///
/// In the old machine architecture, this was AgentReplyMachine:
/// ```ignore
/// ChatEvent::MessageCreated { role: "user", .. } => {
///     Some(GenerateChatReplyCommand::new(message_id, container_id))
/// }
/// ```
///
/// Now it becomes an edge that emits a request event.
pub fn on_message_created(event: &ChatEvent) -> Option<ChatEvent> {
    match event {
        ChatEvent::MessageCreated {
            message_id,
            container_id,
            role,
            ..
        } => {
            // Only trigger AI reply for user messages to prevent loops
            if role == "user" {
                Some(ChatEvent::GenerateReplyRequested {
                    message_id: *message_id,
                    container_id: *container_id,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// React to ContainerCreated by triggering AI greeting when container has an agent.
///
/// When a container is created with with_agent tag, we generate an initial greeting.
/// This edge observes the ContainerCreated fact and emits a GenerateGreetingRequested request.
///
/// In the old machine architecture, this was AgentGreetingMachine:
/// ```ignore
/// ChatEvent::ContainerCreated { with_agent: Some(config), .. } => {
///     Some(GenerateAgentGreetingCommand::new(container_id, config))
/// }
/// ```
pub fn on_container_created(event: &ChatEvent) -> Option<ChatEvent> {
    match event {
        ChatEvent::ContainerCreated {
            container_id,
            with_agent: Some(agent_config),
            ..
        } => Some(ChatEvent::GenerateGreetingRequested {
            container_id: *container_id,
            agent_config: agent_config.clone(),
        }),
        _ => None,
    }
}

/// List of all chat domain internal edges.
///
/// The engine should call each of these when a ChatEvent fact is produced.
pub fn all_edges() -> Vec<fn(&ChatEvent) -> Option<ChatEvent>> {
    vec![on_message_created, on_container_created]
}
