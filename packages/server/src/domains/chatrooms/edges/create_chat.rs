//! CreateChat edge - creates a new AI chat container.

use seesaw_core::{Edge, EdgeContext};

use crate::common::MemberId;
use crate::domains::chatrooms::data::ContainerData;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::state::ChatRequestState;

/// Edge for creating a new AI chat container.
pub struct CreateChat {
    pub language: String,
    pub with_agent: Option<String>,
    pub requested_by: Option<MemberId>,
}

impl Edge<ChatRequestState> for CreateChat {
    type Event = ChatEvent;
    type Data = ContainerData;

    fn execute(&self, _ctx: &EdgeContext<ChatRequestState>) -> Option<ChatEvent> {
        Some(ChatEvent::CreateContainerRequested {
            container_type: "ai_chat".to_string(),
            entity_id: None,
            language: self.language.clone(),
            requested_by: self.requested_by,
            with_agent: self.with_agent.clone(),
        })
    }

    fn read(&self, state: &ChatRequestState) -> Option<ContainerData> {
        state.created_container.clone()
    }
}
