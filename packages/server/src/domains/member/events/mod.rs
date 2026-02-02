use uuid::Uuid;

/// Member domain events
///
/// Request events: From edges (user intent) or internal edges (event reactions)
/// Fact events: From effects (what actually happened)
#[derive(Debug, Clone)]
pub enum MemberEvent {
    // =========================================================================
    // Request events (from edges or internal edges)
    // =========================================================================

    /// User requests to register as a member
    RegisterMemberRequested {
        expo_push_token: String,
        searchable_text: String,
        city: String,
        state: String,
    },

    /// Request to update member's active status
    UpdateMemberStatusRequested { member_id: Uuid, active: bool },

    /// Request to generate embedding for member (triggered by internal edge)
    GenerateEmbeddingRequested { member_id: Uuid },

    // =========================================================================
    // Fact events (from effects)
    // =========================================================================

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

    /// Registration failed
    RegistrationFailed { expo_push_token: String, reason: String },

    /// Embedding was generated successfully
    EmbeddingGenerated { member_id: Uuid, dimensions: usize },

    /// Embedding generation failed
    EmbeddingFailed { member_id: Uuid, reason: String },
}
