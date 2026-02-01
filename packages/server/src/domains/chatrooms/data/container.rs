//! GraphQL data types for chat containers.

use serde::{Deserialize, Serialize};

use crate::domains::chatrooms::models::Container;

/// GraphQL-friendly representation of a chat container
#[derive(Debug, Clone, Serialize, Deserialize, juniper::GraphQLObject)]
#[graphql(description = "A chat container (AI chat, comments, discussions)")]
pub struct ContainerData {
    /// Unique identifier
    pub id: String,

    /// Type of container: ai_chat, post_comments, org_discussion
    pub container_type: String,

    /// Optional entity ID this container is attached to (post_id, org_id, etc)
    pub entity_id: Option<String>,

    /// Language code (e.g., "en", "es")
    pub language: String,

    /// When the container was created (ISO 8601)
    pub created_at: String,

    /// When the last activity occurred (ISO 8601)
    pub last_activity_at: String,
}

impl From<Container> for ContainerData {
    fn from(c: Container) -> Self {
        Self {
            id: c.id.to_string(),
            container_type: c.container_type,
            entity_id: c.entity_id.map(|id| id.to_string()),
            language: c.language,
            created_at: c.created_at.to_rfc3339(),
            last_activity_at: c.last_activity_at.to_rfc3339(),
        }
    }
}
