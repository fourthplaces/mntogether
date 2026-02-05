//! Update member status action

use anyhow::Result;
use tracing::{error, info};
use uuid::Uuid;

use crate::domains::member::events::MemberEvent;
use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Update a member's active status.
///
/// Returns MemberStatusUpdated event on success.
pub async fn update_member_status(
    member_id: Uuid,
    active: bool,
    deps: &ServerDeps,
) -> Result<MemberEvent> {
    info!("Updating member {} status to: {}", member_id, active);

    match Member::update_status(member_id, active, &deps.db_pool).await {
        Ok(updated) => {
            info!("Member status updated: {}", updated.id);
            Ok(MemberEvent::MemberStatusUpdated {
                member_id: updated.id,
                active: updated.active,
            })
        }
        Err(e) => {
            error!("Failed to update member status: {}", e);
            Err(anyhow::anyhow!("Member not found: {}", member_id))
        }
    }
}
