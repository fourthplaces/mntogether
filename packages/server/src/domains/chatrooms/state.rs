//! Chat request state for seesaw Edge pattern.

use crate::domains::chatrooms::data::{ContainerData, MessageData};

/// State passed through engine.run() for chat operations.
#[derive(Clone, Default)]
pub struct ChatRequestState {
    pub created_container: Option<ContainerData>,
    pub created_message: Option<MessageData>,
}
