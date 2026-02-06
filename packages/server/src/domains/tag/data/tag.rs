use chrono::{DateTime, Utc};
use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

use crate::domains::tag::models::Tag;

/// Tag GraphQL data type
#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[graphql(description = "A tag that can be associated with various entities")]
pub struct TagData {
    /// Unique identifier
    pub id: String,

    /// Tag kind (e.g., 'community_served', 'provider_category')
    pub kind: String,

    /// Tag value (e.g., 'somali', 'therapist')
    pub value: String,

    /// Human-readable display name
    pub display_name: Option<String>,

    /// Parent tag ID for hierarchy (e.g., 'Food' > 'Food Pantries')
    pub parent_tag_id: Option<String>,

    /// Code in external taxonomy (e.g., 'BD-1800.2000' for 211HSIS)
    pub external_code: Option<String>,

    /// Taxonomy system: 'custom', 'open_eligibility', '211hsis'
    pub taxonomy_source: Option<String>,

    /// When the tag was created
    pub created_at: DateTime<Utc>,
}

impl From<Tag> for TagData {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id.to_string(),
            kind: tag.kind,
            value: tag.value,
            display_name: tag.display_name,
            parent_tag_id: tag.parent_tag_id.map(|id| id.to_string()),
            external_code: tag.external_code,
            taxonomy_source: tag.taxonomy_source,
            created_at: tag.created_at,
        }
    }
}
