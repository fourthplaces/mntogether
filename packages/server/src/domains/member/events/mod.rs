use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Member domain events - FACT EVENTS ONLY
///
/// These are immutable facts about what happened. Effects watch these
/// and call handlers directly for cascade workflows (no *Requested events).
///
/// NOTE: Failed/error events have been removed (MemberNotFound, EmbeddingFailed).
/// Errors go in Result::Err, not in events. Events are for successful state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberEvent {
    /// Member was successfully registered
    MemberRegistered {
        member_id: Uuid,
        expo_push_token: String,
        latitude: Option<f64>,
        longitude: Option<f64>,
        location_name: Option<String>,
    },

    /// Member status was updated
    MemberStatusUpdated { member_id: Uuid, active: bool },

    /// Embedding was generated successfully
    EmbeddingGenerated { member_id: Uuid, dimensions: usize },
}
