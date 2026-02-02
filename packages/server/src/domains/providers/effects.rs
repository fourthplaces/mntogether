//! Provider effects - handle cascading reactions to fact events
//!
//! Effects watch FACT events and call handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Cascade flow:
//!   ProviderDeleted → cleanup contacts and tags

use seesaw_core::effect;
use seesaw_core::EffectContext;
use std::sync::Arc;
use tracing::info;

use crate::common::AppState;
use crate::domains::contacts::Contact;
use crate::domains::providers::events::ProviderEvent;
use crate::domains::tag::Taggable;
use crate::kernel::ServerDeps;

/// Build the provider effect handler.
///
/// Cascade flow:
///   ProviderDeleted → cleanup contacts and tags
pub fn provider_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ProviderEvent>().run(|event: Arc<ProviderEvent>, ctx: EffectContext<AppState, ServerDeps>| async move {
        match event.as_ref() {
            // =================================================================
            // Cascade: ProviderDeleted → cleanup contacts and tags
            // =================================================================
            ProviderEvent::ProviderDeleted { provider_id } => {
                info!(provider_id = %provider_id, "Cascading provider delete - cleaning up contacts and tags");

                // Clean up contacts
                if let Err(e) =
                    Contact::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await
                {
                    tracing::warn!(
                        provider_id = %provider_id,
                        error = %e,
                        "Failed to delete provider contacts (non-fatal)"
                    );
                }

                // Clean up tags
                if let Err(e) =
                    Taggable::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await
                {
                    tracing::warn!(
                        provider_id = %provider_id,
                        error = %e,
                        "Failed to delete provider tags (non-fatal)"
                    );
                }

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
    })
}
