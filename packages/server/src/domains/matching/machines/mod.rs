mod coordinator;

pub use coordinator::MatchingCoordinatorMachine;

use crate::common::NeedId;
use seesaw::Machine;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::domains::matching::{commands::MatchingCommand, events::MatchingEvent};

/// Matching state machine
///
/// Tracks which needs are currently being matched
pub struct MatchingMachine {
    pending_matches: HashMap<NeedId, ()>, // need_id -> ()
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
                need_id,
                candidate_count,
                notified_count,
            } => {
                info!(
                    need_id = %need_id,
                    candidates = candidate_count,
                    notified = notified_count,
                    "Matching completed successfully"
                );
                self.pending_matches.remove(need_id);
                None
            }

            // No matches found → clear pending (no command needed)
            MatchingEvent::NoMatchesFound { need_id, reason } => {
                warn!(need_id = %need_id, reason = %reason, "No matches found for need");
                self.pending_matches.remove(need_id);
                None
            }

            // Matching failed → clear pending (no command needed)
            MatchingEvent::MatchingFailed { need_id, error } => {
                warn!(need_id = %need_id, error = %error, "Matching failed");
                self.pending_matches.remove(need_id);
                None
            }

            // Manual request → issue command (for future use cases like retries)
            MatchingEvent::FindMatchesRequested { need_id } => {
                debug!(need_id = %need_id, "Manual matching request received");
                self.pending_matches.insert(*need_id, ());
                Some(MatchingCommand::FindMatches { need_id: *need_id })
            }
        }
    }
}

impl Default for MatchingMachine {
    fn default() -> Self {
        Self::new()
    }
}
