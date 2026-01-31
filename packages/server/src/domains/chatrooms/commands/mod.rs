//! Chat domain commands.
//!
//! Commands are requests for IO operations. They follow the seesaw-rs pattern:
//! - Inline commands: Execute immediately
//! - Background commands: Execute via job queue with idempotency

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::common::{ContainerId, MemberId, MessageId};

/// Chat domain commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatCommand {
    /// Create a new container
    CreateContainer {
        container_type: String,
        entity_id: Option<uuid::Uuid>,
        language: String,
        requested_by: Option<MemberId>,
    },

    /// Create a message in a container
    CreateMessage {
        container_id: ContainerId,
        role: String,
        content: String,
        author_id: Option<MemberId>,
        parent_message_id: Option<MessageId>,
    },
}

impl seesaw_core::Command for ChatCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        seesaw_core::ExecutionMode::Inline
    }

    fn job_spec(&self) -> Option<seesaw_core::JobSpec> {
        None
    }

    fn serialize_to_json(&self) -> Option<JsonValue> {
        serde_json::to_value(self).ok()
    }
}

// =============================================================================
// Background Commands
// =============================================================================

/// Job type for agent reply generation
pub const GENERATE_CHAT_REPLY_JOB_TYPE: &str = "generate_chat_reply";

/// Command to generate an agent reply (runs as background job).
///
/// This command is emitted by AgentReplyMachine when a user message is created.
/// The effect generates AI text and returns ChatMessagingEvent::ReplyGenerated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateChatReplyCommand {
    pub message_id: uuid::Uuid,
    pub container_id: uuid::Uuid,
}

impl GenerateChatReplyCommand {
    pub fn new(message_id: MessageId, container_id: ContainerId) -> Self {
        Self {
            message_id: message_id.into_uuid(),
            container_id: container_id.into_uuid(),
        }
    }
}

impl seesaw_core::Command for GenerateChatReplyCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        // For now, run inline since we don't have job queue set up
        // TODO: Switch to Background when job queue is ready
        seesaw_core::ExecutionMode::Inline
    }

    fn job_spec(&self) -> Option<seesaw_core::JobSpec> {
        Some(seesaw_core::JobSpec {
            job_type: GENERATE_CHAT_REPLY_JOB_TYPE,
            idempotency_key: Some(format!(
                "{}:{}",
                GENERATE_CHAT_REPLY_JOB_TYPE, self.message_id
            )),
            max_retries: 3,
            priority: 0,
            version: 1,
        })
    }

    fn serialize_to_json(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self).ok()
    }
}
