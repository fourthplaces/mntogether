//! Resource domain effect - handles cascading reactions to fact events
//!
//! Following the direct-call pattern:
//!   GraphQL → Action → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Currently, resources don't have complex cascades (no related entities to clean up).
//! This effect is in place for future expansion (e.g., notification on approval).

use seesaw_core::effect;
use seesaw_core::EffectContext;
use std::sync::Arc;
use tracing::info;

use crate::common::AppState;
use crate::domains::resources::events::ResourceEvent;
use crate::kernel::ServerDeps;

/// Build the resource effect handler.
///
/// Currently, resource events are terminal (no cascades needed).
/// This effect provides observability and a hook for future cascades.
pub fn resource_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ResourceEvent>().run(
        |event: Arc<ResourceEvent>, _ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Terminal events - log for observability, no cascade needed
                // =================================================================
                ResourceEvent::ResourceApproved { resource_id } => {
                    info!(resource_id = %resource_id, "Resource approved");
                    // Future: Could trigger notification to website owner
                    Ok(())
                }

                ResourceEvent::ResourceRejected {
                    resource_id,
                    reason,
                } => {
                    info!(resource_id = %resource_id, reason = %reason, "Resource rejected");
                    // Future: Could trigger notification to admin
                    Ok(())
                }

                ResourceEvent::ResourceEdited { resource_id } => {
                    info!(resource_id = %resource_id, "Resource edited");
                    Ok(())
                }

                ResourceEvent::ResourceDeleted { resource_id } => {
                    info!(resource_id = %resource_id, "Resource deleted");
                    // Future: Could clean up related data (tags, sources already cascade via FK)
                    Ok(())
                }
            }
        },
    )
}
