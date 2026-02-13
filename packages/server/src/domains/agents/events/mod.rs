//! Agent domain events.

use serde::{Deserialize, Serialize};

/// Chat-specific streaming events.
///
/// Serialized to JSON and published to StreamHub on topic "chat:{container_id}".
/// The SSE endpoint reads the "type" field to set the SSE event name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    /// Generation complete — message has been persisted to DB
    MessageComplete {
        container_id: String,
        message_id: String,
        content: String,
        role: String,
        created_at: String,
    },
}

impl ChatStreamEvent {
    /// The StreamHub topic for a container's chat stream.
    pub fn topic(container_id: &str) -> String {
        format!("chat:{}", container_id)
    }
}
