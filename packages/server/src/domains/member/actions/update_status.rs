//! Update member status action

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};
use uuid::Uuid;

use crate::common::{AppState, ReadResult};
use crate::domains::member::events::MemberEvent;
use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Update a member's active status.
///
/// Called directly from GraphQL mutation via `process()`.
/// Emits `MemberStatusUpdated` fact event on success.
/// Returns `ReadResult<Member>` for deferred read after effects settle.
pub async fn update_member_status(
    member_id: Uuid,
    active: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Member>> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!("Updating member {} status to: {}", member_id, active);

    match Member::update_status(member_id, active, &ctx.deps().db_pool).await {
        Ok(updated) => {
            info!("Member status updated: {}", updated.id);
            ctx.emit(MemberEvent::MemberStatusUpdated {
                member_id: updated.id,
                active: updated.active,
            });
            Ok(ReadResult::new(updated.id, ctx.deps().db_pool.clone()))
        }
        Err(e) => {
            error!("Failed to update member status: {}", e);
            Err(anyhow::anyhow!("Member not found: {}", member_id))
        }
    }
}
