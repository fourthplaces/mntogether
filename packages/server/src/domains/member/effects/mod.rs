use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::common::utils::geocoding::geocode_city;
use crate::domains::listings::effects::ServerDeps;
use crate::domains::member::{
    commands::MemberCommand, events::MemberEvent, models::member::Member,
};

/// Registration effect - handles member registration with geocoding
pub struct RegistrationEffect;

#[async_trait]
impl Effect<MemberCommand, ServerDeps> for RegistrationEffect {
    type Event = MemberEvent;

    async fn execute(
        &self,
        cmd: MemberCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<MemberEvent> {
        match cmd {
            MemberCommand::RegisterMember {
                expo_push_token,
                searchable_text,
                city,
                state,
            } => {
                info!(
                    "Registering member with token: {} in {}, {}",
                    expo_push_token, city, state
                );

                // Check if member already exists
                if let Some(existing) =
                    Member::find_by_token(&expo_push_token, &ctx.deps().db_pool).await?
                {
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

                // Create member
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

            MemberCommand::UpdateMemberStatus { member_id, active } => {
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
                        Ok(MemberEvent::MemberNotFound { member_id })
                    }
                }
            }

            MemberCommand::GenerateEmbedding { member_id } => {
                info!("Generating embedding for member: {}", member_id);

                // Get member from database
                let member = match Member::find_by_id(member_id, &ctx.deps().db_pool).await {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Member not found: {}", e);
                        return Ok(MemberEvent::EmbeddingFailed {
                            member_id,
                            reason: format!("Member not found: {}", e),
                        });
                    }
                };

                // Generate embedding
                let embedding = match ctx
                    .deps()
                    .embedding_service
                    .generate(&member.searchable_text)
                    .await
                {
                    Ok(emb) => emb,
                    Err(e) => {
                        error!("Failed to generate embedding: {}", e);
                        return Ok(MemberEvent::EmbeddingFailed {
                            member_id,
                            reason: format!("Embedding generation failed: {}", e),
                        });
                    }
                };

                debug!("Generated embedding with {} dimensions", embedding.len());

                // Update member with embedding
                if let Err(e) =
                    Member::update_embedding(member_id, &embedding, &ctx.deps().db_pool).await
                {
                    error!("Failed to save embedding: {}", e);
                    return Ok(MemberEvent::EmbeddingFailed {
                        member_id,
                        reason: format!("Failed to save embedding: {}", e),
                    });
                }

                info!("Embedding generated and saved for member: {}", member_id);

                Ok(MemberEvent::EmbeddingGenerated {
                    member_id,
                    dimensions: embedding.len(),
                })
            }
        }
    }
}
