//! Register member action

use anyhow::Result;
use tracing::{debug, info};
use uuid::Uuid;

use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Register a new member.
///
/// Returns the member ID (new or existing).
pub async fn register_member(
    expo_push_token: String,
    searchable_text: String,
    city: String,
    state: String,
    deps: &ServerDeps,
) -> Result<Uuid> {
    info!(
        "Registering member with token: {} in {}, {}",
        expo_push_token, city, state
    );

    // Check if member already exists (idempotency)
    if let Some(existing) = Member::find_by_token(&expo_push_token, &deps.db_pool).await? {
        debug!("Member already exists, returning existing: {}", existing.id);
        return Ok(existing.id);
    }

    // Create member record
    let member = Member {
        id: Uuid::new_v4(),
        expo_push_token: expo_push_token.clone(),
        searchable_text,
        latitude: None,
        longitude: None,
        location_name: None,
        active: true,
        notification_count_this_week: 0,
        paused_until: None,
        created_at: chrono::Utc::now(),
    };

    let created = member.insert(&deps.db_pool).await?;

    info!("Member registered successfully: {}", created.id);

    Ok(created.id)
}
