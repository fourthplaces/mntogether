use uuid::Uuid;

/// Matching domain events
#[derive(Debug, Clone)]
pub enum MatchingEvent {
    // Request events (from edges or organization domain)
    FindMatchesRequested {
        need_id: Uuid,
    },

    // Fact events (from effects)
    MatchesFound {
        need_id: Uuid,
        candidate_count: usize,
        notified_count: usize,
    },
    NoMatchesFound {
        need_id: Uuid,
        reason: String,
    },
    MatchingFailed {
        need_id: Uuid,
        error: String,
    },
}

impl seesaw::Event for MatchingEvent {}
