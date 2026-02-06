//! Member domain effect - handles cascading reactions to fact events
//!
//! Cascade flow:
//!   MemberRegistered → generate embedding (queued)

use anyhow::Result;
use seesaw_core::{effect, effects, EffectContext};
use tracing::info;
use uuid::Uuid;

use super::actions;
use super::events::MemberEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

#[effects]
pub mod handlers {
    use super::*;

    #[effect(
        on = [MemberEvent::MemberRegistered],
        extract(member_id),
        id = "member_embedding",
        retry = 3,
        timeout_secs = 30
    )]
    async fn generate_member_embedding(
        member_id: Uuid,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
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
    }
}
