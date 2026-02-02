//! Register member action - handles member registration with geocoding

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::common::utils::geocoding::geocode_city;
use crate::domains::member::events::MemberEvent;
use crate::domains::member::models::member::Member;
use crate::domains::posts::effects::ServerDeps;

/// Register a new member with geocoding.
///
/// This action:
/// 1. Checks if member already exists (by expo_push_token)
/// 2. Geocodes the city/state to lat/lng
/// 3. Creates the member record
///
/// Returns:
/// - `MemberRegistered` on success (new or existing member)
/// - Error propagated on database failure
pub async fn register_member(
    expo_push_token: String,
    searchable_text: String,
    city: String,
    state: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<MemberEvent> {
    info!(
        "Registering member with token: {} in {}, {}",
        expo_push_token, city, state
    );

    // Check if member already exists (idempotency)
    if let Some(existing) = Member::find_by_token(&expo_push_token, &ctx.deps().db_pool).await? {
        debug!("Member already exists, returning existing: {}", existing.id);
        return Ok(MemberEvent::MemberRegistered {
            member_id: existing.id,
            expo_push_token: existing.expo_push_token,
            latitude: existing.latitude,
            longitude: existing.longitude,
            location_name: existing.location_name,
        });
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
            // Don't fail registration, just skip location
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
        location_name,
        active: true,
        notification_count_this_week: 0,
        paused_until: None,
        created_at: chrono::Utc::now(),
    };

    // Insert into database
    let created = member.insert(&ctx.deps().db_pool).await.map_err(|e| {
        error!("Failed to insert member: {}", e);
        anyhow::anyhow!("Database error: {}", e)
    })?;

    info!("Member registered successfully: {}", created.id);

    Ok(MemberEvent::MemberRegistered {
        member_id: created.id,
        expo_push_token: created.expo_push_token,
        latitude: created.latitude,
        longitude: created.longitude,
        location_name: created.location_name,
    })
}
