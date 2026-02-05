//! Member domain effect - handles cascading reactions to fact events
//!
//! Effects use `.then()` and return `Ok(())` for terminal.
//!
//! Cascade flow:
//!   MemberRegistered → generate embedding (terminal)

use seesaw_core::{effect, EffectContext};
use tracing::{error, info};

use super::actions;
use super::events::MemberEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

/// Build the member effect handler.
///
/// Cascade flow:
///   MemberRegistered → generate embedding (terminal)
pub fn member_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<MemberEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: MemberRegistered → generate embedding
                // =================================================================
                MemberEvent::MemberRegistered { member_id, .. } => {
                    // Call the action to generate embedding
                    match actions::generate_embedding(
                        *member_id,
                        ctx.deps().embedding_service.as_ref(),
                        &ctx.deps().db_pool,
                    )
                    .await
                    {
                        Ok(result) => {
                            info!(
                                member_id = %result.member_id,
                                dimensions = result.dimensions,
                                "Embedding generated for member"
                            );
                        }
                        Err(e) => {
                            error!(
                                member_id = %member_id,
                                error = %e,
                                "Embedding generation failed for member (non-fatal)"
                            );
                        }
                    }
                    Ok(()) // Terminal
                }

                // =================================================================
                // Terminal events - no cascade needed
                // =================================================================
                MemberEvent::MemberStatusUpdated { .. }
                | MemberEvent::EmbeddingGenerated { .. } => Ok(()),
            }
        },
    )
}
