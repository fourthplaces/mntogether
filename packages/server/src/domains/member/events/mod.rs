use uuid::Uuid;

/// Member domain events - FACT EVENTS ONLY
///
/// These are immutable facts about what happened. Effects watch these
/// and call handlers directly for cascade workflows (no *Requested events).
#[derive(Debug, Clone)]
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

    /// Member was not found
    MemberNotFound { member_id: Uuid },

    /// Embedding was generated successfully
    EmbeddingGenerated { member_id: Uuid, dimensions: usize },

    /// Embedding generation failed
    EmbeddingFailed { member_id: Uuid, reason: String },
}
