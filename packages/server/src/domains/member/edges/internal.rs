//! Member domain internal edges - event-to-event reactions
//!
//! Internal edges observe fact events and emit new request events.
//! This replaces the machine's decide() logic in seesaw 0.3.0.
//!
//! Flow:
//!   Fact Event → Internal Edge → Option<Request Event>
//!
//! The engine calls these edges after effects produce fact events.
//! If an edge returns Some(event), that event is dispatched to effects.

use crate::domains::member::events::MemberEvent;

/// React to MemberRegistered by triggering embedding generation.
///
/// When a member is registered, we want to generate their embedding
/// in the background. This edge observes the MemberRegistered fact
/// and emits a GenerateEmbeddingRequested request.
///
/// In the old machine architecture, this was:
/// ```ignore
/// MemberEvent::MemberRegistered { member_id, .. } => {
///     Some(MemberCommand::GenerateEmbedding { member_id: *member_id })
/// }
/// ```
///
/// Now it becomes an edge that emits a request event:
pub fn on_member_registered(event: &MemberEvent) -> Option<MemberEvent> {
    match event {
        MemberEvent::MemberRegistered { member_id, .. } => {
            // Trigger embedding generation for the newly registered member
            Some(MemberEvent::GenerateEmbeddingRequested {
                member_id: *member_id,
            })
        }
        _ => None,
    }
}

/// React to RegistrationFailed - currently no action needed.
///
/// The old machine cleared pending_registrations state here.
/// In 0.3.0, we don't track that state (it was only for deduplication).
/// If we need deduplication, we can add idempotency at the action level.
pub fn on_registration_failed(event: &MemberEvent) -> Option<MemberEvent> {
    match event {
        MemberEvent::RegistrationFailed { .. } => {
            // No follow-up action needed
            None
        }
        _ => None,
    }
}
