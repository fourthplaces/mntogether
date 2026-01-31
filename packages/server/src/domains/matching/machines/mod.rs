mod coordinator;

pub use coordinator::MatchingCoordinatorMachine;

use crate::common::ListingId;
use seesaw_core::Machine;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::domains::matching::{commands::MatchingCommand, events::MatchingEvent};

/// Matching state machine
///
/// Tracks which listings are currently being matched
pub struct MatchingMachine {
    pending_matches: HashMap<ListingId, ()>, // listing_id -> ()
}

impl MatchingMachine {
    pub fn new() -> Self {
        Self {
            pending_matches: HashMap::new(),
        }
    }
}

impl Machine for MatchingMachine {
    type Event = MatchingEvent;
    type Command = MatchingCommand;

    fn decide(&mut self, event: &MatchingEvent) -> Option<MatchingCommand> {
        match event {
            // Matching completed → clear pending (no command needed)
            MatchingEvent::MatchesFound {
                listing_id,
                candidate_count,
                notified_count,
            } => {
                info!(
                    listing_id = %listing_id,
                    candidates = candidate_count,
                    notified = notified_count,
                    "Matching completed successfully"
                );
                self.pending_matches.remove(listing_id);
                None
            }

            // No matches found → clear pending (no command needed)
            MatchingEvent::NoMatchesFound { listing_id, reason } => {
                warn!(listing_id = %listing_id, reason = %reason, "No matches found for listing");
                self.pending_matches.remove(listing_id);
                None
            }

            // Matching failed → clear pending (no command needed)
            MatchingEvent::MatchingFailed { listing_id, error } => {
                warn!(listing_id = %listing_id, error = %error, "Matching failed");
                self.pending_matches.remove(listing_id);
                None
            }

            // Manual request → issue command (for future use cases like retries)
            MatchingEvent::FindMatchesRequested { listing_id } => {
                debug!(listing_id = %listing_id, "Manual matching request received");
                self.pending_matches.insert(*listing_id, ());
                Some(MatchingCommand::FindMatches {
                    listing_id: *listing_id,
                })
            }
        }
    }
}

impl Default for MatchingMachine {
    fn default() -> Self {
        Self::new()
    }
}
