//! Agent domain events.

use serde::{Deserialize, Serialize};

/// Chat-specific streaming events.
///
/// Serialized to JSON and published to StreamHub on topic "chat:{container_id}".
/// The SSE endpoint reads the "type" field to set the SSE event name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    /// AI generation has started for a message
    GenerationStarted {
        container_id: String,
        in_reply_to: String,
    },

    /// A tool produced results (e.g., search_posts returned resources)
    ToolResult {
        container_id: String,
        tool_name: String,
        call_id: String,
        results: serde_json::Value,
    },

    /// A token chunk from the streaming response
    TokenDelta { container_id: String, delta: String },

    /// Generation complete — message has been persisted to DB
    MessageComplete {
        container_id: String,
        message_id: String,
        content: String,
        role: String,
        created_at: String,
    },

    /// Generation failed
    GenerationError { container_id: String, error: String },
}

impl ChatStreamEvent {
    /// The StreamHub topic for a container's chat stream.
    pub fn topic(container_id: &str) -> String {
        format!("chat:{}", container_id)
    }
}
