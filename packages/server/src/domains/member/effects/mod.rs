//! Member domain effect - thin dispatcher to actions
//!
//! The effect handles request events and dispatches to action functions.
//! All business logic lives in the actions module.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::actions;
use super::events::MemberEvent;
use crate::domains::posts::effects::ServerDeps;

/// Member effect - handles MemberEvent request events
///
/// Request events are dispatched to actions which contain the business logic.
/// Fact events should never reach this effect (they're outputs, not inputs).
pub struct MemberEffect;

#[async_trait]
impl Effect<MemberEvent, ServerDeps> for MemberEffect {
    type Event = MemberEvent;

    async fn handle(
        &mut self,
        event: MemberEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<MemberEvent> {
        match event {
            // =================================================================
            // Request Events → Dispatch to Actions
            // =================================================================
            MemberEvent::RegisterMemberRequested {
                expo_push_token,
                searchable_text,
                city,
                state,
            } => {
                actions::register_member(expo_push_token, searchable_text, city, state, &ctx).await
            }

            MemberEvent::UpdateMemberStatusRequested { member_id, active } => {
                actions::update_member_status(member_id, active, &ctx).await
            }

            MemberEvent::GenerateEmbeddingRequested { member_id } => {
                actions::generate_embedding(member_id, &ctx).await
            }

            // =================================================================
            // Fact Events → Should not reach effect (return error)
            // =================================================================
            MemberEvent::MemberRegistered { .. }
            | MemberEvent::MemberStatusUpdated { .. }
            | MemberEvent::MemberNotFound { .. }
            | MemberEvent::RegistrationFailed { .. }
            | MemberEvent::EmbeddingGenerated { .. }
            | MemberEvent::EmbeddingFailed { .. } => {
                anyhow::bail!(
                    "Fact events should not be dispatched to effects. \
                     They are outputs from effects, not inputs."
                )
            }
        }
    }
}
