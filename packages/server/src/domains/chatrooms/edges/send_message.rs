//! SendMessage edge - sends a message to a chat container.

use seesaw_core::{Edge, EdgeContext};

use crate::common::{ContainerId, MemberId};
use crate::domains::chatrooms::data::MessageData;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::state::ChatRequestState;

/// Edge for sending a message to a chat container.
pub struct SendMessage {
    pub container_id: ContainerId,
    pub content: String,
    pub author_id: Option<MemberId>,
}

impl Edge<ChatRequestState> for SendMessage {
    type Event = ChatEvent;
    type Data = MessageData;

    fn execute(&self, _ctx: &EdgeContext<ChatRequestState>) -> Option<ChatEvent> {
        Some(ChatEvent::SendMessageRequested {
            container_id: self.container_id,
            content: self.content.clone(),
            author_id: self.author_id,
            parent_message_id: None,
        })
    }

    fn read(&self, state: &ChatRequestState) -> Option<MessageData> {
        state.created_message.clone()
    }
}
