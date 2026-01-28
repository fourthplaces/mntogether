use uuid::Uuid;

/// Member domain events
///
/// Request events: From edges (user intent)
/// Fact events: From effects (what actually happened)
#[derive(Debug, Clone)]
pub enum MemberEvent {
    // Request events (from edges)
    RegisterMemberRequested {
        expo_push_token: String,
        searchable_text: String,
        city: String,
        state: String,
    },
    UpdateMemberStatusRequested {
        member_id: Uuid,
        active: bool,
    },

    // Fact events (from effects)
    MemberRegistered {
        member_id: Uuid,
        expo_push_token: String,
        latitude: Option<f64>,
        longitude: Option<f64>,
        location_name: Option<String>,
    },
    MemberStatusUpdated {
        member_id: Uuid,
        active: bool,
    },
    MemberNotFound {
        member_id: Uuid,
    },
    RegistrationFailed {
        expo_push_token: String,
        reason: String,
    },
    EmbeddingGenerated {
        member_id: Uuid,
        dimensions: usize,
    },
    EmbeddingFailed {
        member_id: Uuid,
        reason: String,
    },
}

// Auto-implement Event trait (from seesaw)
// Event is auto-implemented for Clone + Send + Sync types
