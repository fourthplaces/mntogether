//! Member domain effect - handles cascading reactions to fact events
//!
//! Effects watch FACT events and call handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.

use seesaw_core::effect;
use std::sync::Arc;

use super::actions;
use super::events::MemberEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

/// Build the member effect handler.
///
/// Cascade flow:
///   MemberRegistered → handle_generate_embedding → EmbeddingGenerated
pub fn member_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<MemberEvent>().run(|event: Arc<MemberEvent>, ctx| async move {
        match event.as_ref() {
            // =================================================================
            // Cascade: MemberRegistered → generate embedding
            // =================================================================
            MemberEvent::MemberRegistered { member_id, .. } => {
                actions::handle_generate_embedding(*member_id, &ctx).await
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            MemberEvent::MemberStatusUpdated { .. }
            | MemberEvent::MemberNotFound { .. }
            | MemberEvent::EmbeddingGenerated { .. }
            | MemberEvent::EmbeddingFailed { .. } => Ok(()),
        }
    })
}
