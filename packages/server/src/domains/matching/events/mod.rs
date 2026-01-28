use crate::common::NeedId;

/// Matching domain events
#[derive(Debug, Clone)]
pub enum MatchingEvent {
    // Request events (from edges or organization domain)
    FindMatchesRequested {
        need_id: NeedId,
    },

    // Fact events (from effects)
    MatchesFound {
        need_id: NeedId,
        candidate_count: usize,
        notified_count: usize,
    },
    NoMatchesFound {
        need_id: NeedId,
        reason: String,
    },
    MatchingFailed {
        need_id: NeedId,
        error: String,
    },
}

// Event is auto-implemented for Clone + Send + Sync types
