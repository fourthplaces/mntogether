//! Chat reducer - updates state from events.

use seesaw_core::Reducer;

use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::state::ChatRequestState;

/// Reducer that stores created container/message in state.
pub struct ChatReducer;

impl Reducer<ChatEvent, ChatRequestState> for ChatReducer {
    fn reduce(&self, state: &ChatRequestState, event: &ChatEvent) -> ChatRequestState {
        match event {
            ChatEvent::ContainerCreated { container, .. } => ChatRequestState {
                created_container: Some(ContainerData::from(container.clone())),
                ..state.clone()
            },
            ChatEvent::MessageCreated { message, .. } => ChatRequestState {
                created_message: Some(MessageData::from(message.clone())),
                ..state.clone()
            },
            _ => state.clone(),
        }
    }
}
