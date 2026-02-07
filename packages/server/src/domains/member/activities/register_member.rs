//! Register member action - handles member registration with geocoding

use anyhow::Result;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::common::utils::geocoding::geocode_city;
use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Register a new member with geocoding.
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

    // Geocode city to lat/lng
    let (latitude, longitude, location_name) = match geocode_city(&city, &state).await {
        Ok(location) => (
            Some(location.latitude),
            Some(location.longitude),
            Some(location.display_name),
        ),
        Err(e) => {
            error!("Geocoding failed for {}, {}: {}", city, state, e);
            (None, None, None)
        }
    };

    debug!(
        "Geocoded {}, {} → ({:?}, {:?})",
        city, state, latitude, longitude
    );

    // Create member record
    let member = Member {
        id: Uuid::new_v4(),
        expo_push_token: expo_push_token.clone(),
        searchable_text,
        latitude,
        longitude,
        location_name: location_name.clone(),
        active: true,
        notification_count_this_week: 0,
        paused_until: None,
        created_at: chrono::Utc::now(),
    };

    // Insert into database
    let created = member.insert(&deps.db_pool).await.map_err(|e| {
        error!("Failed to insert member: {}", e);
        anyhow::anyhow!("Database error: {}", e)
    })?;

    info!("Member registered successfully: {}", created.id);

    Ok(created.id)
}
