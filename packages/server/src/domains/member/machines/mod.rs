use seesaw::Machine;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domains::member::{commands::MemberCommand, events::MemberEvent};

/// Member state machine - pure decision logic
///
/// Tracks pending operations and decides on commands based on events
pub struct MemberMachine {
    pending_registrations: HashMap<String, ()>, // expo_push_token -> ()
}

impl MemberMachine {
    pub fn new() -> Self {
        Self {
            pending_registrations: HashMap::new(),
        }
    }
}

impl Machine for MemberMachine {
    type Event = MemberEvent;
    type Command = MemberCommand;

    fn decide(&mut self, event: &MemberEvent) -> Option<MemberCommand> {
        match event {
            // Registration requested → issue command
            MemberEvent::RegisterMemberRequested {
                expo_push_token,
                searchable_text,
                city,
                state,
            } => {
                self.pending_registrations
                    .insert(expo_push_token.clone(), ());

                Some(MemberCommand::RegisterMember {
                    expo_push_token: expo_push_token.clone(),
                    searchable_text: searchable_text.clone(),
                    city: city.clone(),
                    state: state.clone(),
                })
            }

            // Registration completed → clear pending, generate embedding
            MemberEvent::MemberRegistered {
                expo_push_token,
                member_id,
                ..
            } => {
                self.pending_registrations.remove(expo_push_token);
                // Trigger background embedding generation
                Some(MemberCommand::GenerateEmbedding {
                    member_id: *member_id,
                })
            }

            // Registration failed → clear pending
            MemberEvent::RegistrationFailed {
                expo_push_token, ..
            } => {
                self.pending_registrations.remove(expo_push_token);
                None
            }

            // Update status requested → issue command
            MemberEvent::UpdateMemberStatusRequested { member_id, active } => {
                Some(MemberCommand::UpdateMemberStatus {
                    member_id: *member_id,
                    active: *active,
                })
            }

            // Status updated → no further action
            MemberEvent::MemberStatusUpdated { .. } => None,

            // Member not found → no action
            MemberEvent::MemberNotFound { .. } => None,

            // Embedding generated → no further action
            MemberEvent::EmbeddingGenerated { .. } => None,

            // Embedding failed → no retry (job queue will handle)
            MemberEvent::EmbeddingFailed { .. } => None,
        }
    }
}

impl Default for MemberMachine {
    fn default() -> Self {
        Self::new()
    }
}
