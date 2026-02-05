//! Provider effects - handle cascading reactions to fact events
//!
//! Effects use `.then()` and return `Ok(Event)` to chain or `Ok(())` for terminal.
//!
//! Cascade flow:
//!   ProviderDeleted → cleanup contacts and tags (terminal)

use seesaw_core::{effect, EffectContext};
use tracing::info;

use crate::common::AppState;
use crate::domains::contacts::Contact;
use crate::domains::providers::events::ProviderEvent;
use crate::domains::tag::Taggable;
use crate::kernel::ServerDeps;

/// Build the provider effect handler.
///
/// Cascade flow:
///   ProviderDeleted → cleanup contacts and tags (terminal)
/// Errors propagate to global on_error() handler.
pub fn provider_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ProviderEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: ProviderDeleted → cleanup contacts and tags
                // =================================================================
                ProviderEvent::ProviderDeleted { provider_id } => {
                    info!(provider_id = %provider_id, "Cascading provider delete - cleaning up contacts and tags");

                    Contact::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await?;
                    Taggable::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await?;

                    info!(provider_id = %provider_id, "Provider cascade cleanup completed");
                    Ok(())
                }

                // =================================================================
                // Terminal events - no cascade needed
                // =================================================================
                ProviderEvent::ProviderCreated { .. }
                | ProviderEvent::ProviderApproved { .. }
                | ProviderEvent::ProviderRejected { .. }
                | ProviderEvent::ProviderSuspended { .. } => Ok(()),
            }
        },
    )
}
