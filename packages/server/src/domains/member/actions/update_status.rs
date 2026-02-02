//! Update member status action

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};
use uuid::Uuid;

use crate::domains::chatrooms::ChatRequestState;
use crate::domains::member::events::MemberEvent;
use crate::domains::member::models::member::Member;
use crate::domains::posts::effects::ServerDeps;

/// Update a member's active status.
///
/// Returns:
/// - `MemberStatusUpdated` on success
/// - `MemberNotFound` if member doesn't exist
pub async fn update_member_status(
    member_id: Uuid,
    active: bool,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<MemberEvent> {
    info!("Updating member {} status to: {}", member_id, active);

    match Member::update_status(member_id, active, &ctx.deps().db_pool).await {
        Ok(updated) => {
            info!("Member status updated: {}", updated.id);
            Ok(MemberEvent::MemberStatusUpdated {
                member_id: updated.id,
                active: updated.active,
            })
        }
        Err(e) => {
            error!("Failed to update member status: {}", e);
            // Return MemberNotFound as a fact event (not an error)
            // This allows the caller to handle it appropriately
            Ok(MemberEvent::MemberNotFound { member_id })
        }
    }
}
