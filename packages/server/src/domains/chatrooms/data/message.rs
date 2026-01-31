//! GraphQL data types for chat messages.

use serde::{Deserialize, Serialize};

use crate::domains::chatrooms::models::Message;

/// GraphQL-friendly representation of a chat message
#[derive(Debug, Clone, Serialize, Deserialize, juniper::GraphQLObject)]
#[graphql(description = "A message in a chat container")]
pub struct MessageData {
    /// Unique identifier
    pub id: String,

    /// Container this message belongs to
    pub container_id: String,

    /// Role: user, assistant, comment
    pub role: String,

    /// Message content
    pub content: String,

    /// Optional author member ID
    pub author_id: Option<String>,

    /// Moderation status: approved, pending, flagged, removed
    pub moderation_status: String,

    /// Optional parent message ID (for threads)
    pub parent_message_id: Option<String>,

    /// Sequence number within the container
    pub sequence_number: i32,

    /// When the message was created (ISO 8601)
    pub created_at: String,

    /// When the message was last updated (ISO 8601)
    pub updated_at: String,

    /// When the message was edited (ISO 8601), if applicable
    pub edited_at: Option<String>,
}

impl From<Message> for MessageData {
    fn from(m: Message) -> Self {
        Self {
            id: m.id.to_string(),
            container_id: m.container_id.to_string(),
            role: m.role,
            content: m.content,
            author_id: m.author_id.map(|id| id.to_string()),
            moderation_status: m.moderation_status,
            parent_message_id: m.parent_message_id.map(|id| id.to_string()),
            sequence_number: m.sequence_number,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
            edited_at: m.edited_at.map(|dt| dt.to_rfc3339()),
        }
    }
}
