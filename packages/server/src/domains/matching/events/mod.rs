use crate::common::ListingId;

/// Matching domain events
#[derive(Debug, Clone)]
pub enum MatchingEvent {
    // Request events (from edges or organization domain)
    FindMatchesRequested {
        listing_id: ListingId,
    },

    // Fact events (from effects)
    MatchesFound {
        listing_id: ListingId,
        candidate_count: usize,
        notified_count: usize,
    },
    NoMatchesFound {
        listing_id: ListingId,
        reason: String,
    },
    MatchingFailed {
        listing_id: ListingId,
        error: String,
    },
}

// Event is auto-implemented for Clone + Send + Sync types
