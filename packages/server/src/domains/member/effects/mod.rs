//! Member domain effect - handles cascading reactions to fact events
//!
//! Effects use `.then()` and return `Ok(())` for terminal.
//!
//! Cascade flow:
//!   MemberRegistered → generate embedding (terminal)

use seesaw_core::{effect, EffectContext};
use tracing::info;

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
                    let result = actions::generate_embedding(
                        *member_id,
                        ctx.deps().embedding_service.as_ref(),
                        &ctx.deps().db_pool,
                    )
                    .await?;

                    info!(
                        member_id = %result.member_id,
                        dimensions = result.dimensions,
                        "Embedding generated for member"
                    );
                    Ok(())
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
