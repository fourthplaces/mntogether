//! Update member status action

use anyhow::Result;
use tracing::{error, info};
use uuid::Uuid;

use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Update a member's active status.
///
/// Returns the updated member's ID.
pub async fn update_member_status(
    member_id: Uuid,
    active: bool,
    deps: &ServerDeps,
) -> Result<Uuid> {
    info!("Updating member {} status to: {}", member_id, active);

    match Member::update_status(member_id, active, &deps.db_pool).await {
        Ok(updated) => {
            info!("Member status updated: {}", updated.id);
            Ok(updated.id)
        }
        Err(e) => {
            error!("Failed to update member status: {}", e);
            Err(anyhow::anyhow!("Member not found: {}", member_id))
        }
    }
}
