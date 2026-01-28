use crate::domains::matching::commands::MatchingCommand;
use crate::domains::organization::events::OrganizationEvent;
use seesaw::Machine;
use tracing::{debug, info};

/// Coordinator machine that bridges Organization → Matching domains
/// Listens to OrganizationEvent, emits MatchingCommand
pub struct MatchingCoordinatorMachine;

impl MatchingCoordinatorMachine {
    pub fn new() -> Self {
        Self
    }
}

impl Machine for MatchingCoordinatorMachine {
    type Event = OrganizationEvent;
    type Command = MatchingCommand;

    fn decide(&mut self, event: &OrganizationEvent) -> Option<MatchingCommand> {
        match event {
            // When a need is approved, trigger matching
            OrganizationEvent::NeedApproved { need_id } => {
                info!(need_id = %need_id, "Need approved, triggering member matching");
                Some(MatchingCommand::FindMatches { need_id: *need_id })
            }
            _ => {
                debug!(event = ?event, "Ignoring organization event (not relevant for matching)");
                None
            }
        }
    }
}
