//! Provider effects - handle cascading reactions to fact events
//!
//! Effects use `.then()` and return `Ok(Event)` to chain or `Ok(())` for terminal.
//!
//! Cascade flow:
//!   ProviderDeleted → cleanup tags (terminal)

use anyhow::Result;
use seesaw_core::{effect, effects, EffectContext};
use tracing::info;

use crate::common::AppState;
use crate::domains::providers::events::ProviderEvent;
use crate::domains::tag::Taggable;
use crate::kernel::ServerDeps;

#[effects]
pub mod handlers {
    use super::*;

    #[effect(on = ProviderEvent, id = "provider_cleanup")]
    async fn provider_cleanup(
        event: ProviderEvent,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        if let ProviderEvent::ProviderDeleted { provider_id } = &event {
            info!(provider_id = %provider_id, "Cascading provider delete - cleaning up tags");

            Taggable::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await?;

            info!(provider_id = %provider_id, "Provider cascade cleanup completed");
        }
        Ok(())
    }
}
