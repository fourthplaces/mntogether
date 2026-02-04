//! Member domain effect - handles cascading reactions to fact events
//!
//! Effects watch FACT events and call handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Handlers emit events - they wrap actions which return simple values.

use anyhow::Result;
use seesaw_core::effect;
use seesaw_core::EffectContext;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

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
                handle_generate_embedding(*member_id, &ctx).await
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            MemberEvent::MemberStatusUpdated { .. } | MemberEvent::EmbeddingGenerated { .. } => {
                Ok(())
            }
        }
    })
}

// ============================================================================
// Effect Handlers - wrap actions and emit events
// ============================================================================

/// Handle embedding generation - wraps the action and emits events.
///
/// This is a handler (lives in effects) because it emits events.
/// The actual work is done by `actions::generate_embedding`.
async fn handle_generate_embedding(
    member_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    // Call the action
    let result = actions::generate_embedding(
        member_id,
        ctx.deps().embedding_service.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    .map_err(|e| {
        error!(
            "Embedding generation failed for member {}: {}",
            member_id, e
        );
        anyhow::anyhow!("Embedding generation failed: {}", e)
    })?;

    ctx.emit(MemberEvent::EmbeddingGenerated {
        member_id: result.member_id,
        dimensions: result.dimensions,
    });
    Ok(())
}
