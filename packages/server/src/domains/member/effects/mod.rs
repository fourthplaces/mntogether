//! Member domain effect - handles cascading reactions to fact events
//!
//! Cascade flow:
//!   MemberRegistered → generate embedding (queued)

use std::time::Duration;

use seesaw_core::effect;
use tracing::info;

use super::actions;
use super::events::MemberEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

/// Build the member effect handler.
///
/// MemberRegistered → generate embedding (queued, retries 3x)
pub fn member_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<MemberEvent>()
        .extract(|event| match event {
            MemberEvent::MemberRegistered { member_id, .. } => Some(*member_id),
            _ => None,
        })
        .id("member_embedding")
        .queued()
        .retry(3)
        .timeout(Duration::from_secs(30))
        .then(
            |member_id, ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                let result = actions::generate_embedding(
                    member_id,
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
            },
        )
}
