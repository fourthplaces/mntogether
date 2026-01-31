use crate::domains::listings::events::ListingEvent;
use crate::domains::matching::commands::MatchingCommand;
use seesaw_core::Machine;
use tracing::info;

/// Coordinator machine that bridges Listings → Matching domains
/// Listens to ListingEvent, emits MatchingCommand
pub struct MatchingCoordinatorMachine;

impl MatchingCoordinatorMachine {
    pub fn new() -> Self {
        Self
    }
}

impl Machine for MatchingCoordinatorMachine {
    type Event = ListingEvent;
    type Command = MatchingCommand;

    fn decide(&mut self, event: &ListingEvent) -> Option<MatchingCommand> {
        match event {
            // When a listing is approved, trigger matching
            ListingEvent::ListingApproved { listing_id } => {
                info!(listing_id = %listing_id, "Listing approved, triggering member matching");
                Some(MatchingCommand::FindMatches {
                    listing_id: *listing_id,
                })
            }
            _ => None,
        }
    }
}
