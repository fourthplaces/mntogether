use chrono::{DateTime, Utc};
use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

use crate::domains::member::models::member::Member as MemberModel;

/// Member GraphQL data type
///
/// Public API representation of a member (for GraphQL responses)
#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[graphql(description = "A member who can receive volunteer opportunity notifications")]
pub struct MemberData {
    /// Unique identifier
    pub id: String,

    /// Expo push notification token (for sending notifications)
    pub expo_push_token: String,

    /// TEXT-FIRST: All capabilities, skills, interests in searchable text
    pub searchable_text: String,

    /// Coarse latitude (city-level precision)
    pub latitude: Option<f64>,

    /// Coarse longitude (city-level precision)
    pub longitude: Option<f64>,

    /// Human-readable location name (e.g., "Minneapolis, MN")
    pub location_name: Option<String>,

    /// Whether member is active (receiving notifications)
    pub active: bool,

    /// Number of notifications sent this week (max 3)
    pub notification_count_this_week: i32,

    /// When member registered
    pub created_at: DateTime<Utc>,
}

impl From<MemberModel> for MemberData {
    fn from(member: MemberModel) -> Self {
        Self {
            id: member.id.to_string(),
            expo_push_token: member.expo_push_token,
            searchable_text: member.searchable_text,
            latitude: member.latitude,
            longitude: member.longitude,
            location_name: member.location_name,
            active: member.active,
            notification_count_this_week: member.notification_count_this_week,
            created_at: member.created_at,
        }
    }
}
