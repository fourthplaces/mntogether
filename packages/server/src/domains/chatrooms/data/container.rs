//! GraphQL data types for chat containers.

use serde::{Deserialize, Serialize};

use crate::domains::chatrooms::models::Container;

/// GraphQL-friendly representation of a chat container
#[derive(Debug, Clone, Serialize, Deserialize, juniper::GraphQLObject)]
#[graphql(description = "A chat container (AI chat, comments, discussions)")]
pub struct ContainerData {
    /// Unique identifier
    pub id: String,

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
            language: c.language,
            created_at: c.created_at.to_rfc3339(),
            last_activity_at: c.last_activity_at.to_rfc3339(),
        }
    }
}
