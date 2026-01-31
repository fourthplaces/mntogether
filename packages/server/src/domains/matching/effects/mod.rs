pub mod vector_search;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use seesaw_core::{Effect, EffectContext};
use sqlx::PgPool;
use tracing::{debug, error, info, instrument, warn};

use crate::common::{ListingId, MemberId};
use crate::domains::listings::effects::ServerDeps;
use crate::domains::listings::models::listing::Listing;
use crate::domains::matching::{
    commands::MatchingCommand, events::MatchingEvent, models::notification::Notification,
    utils::check_relevance_by_similarity,
};
use crate::domains::member::models::member::Member;

use vector_search::MatchCandidate;

/// Default search radius in kilometers (≈ 20 miles)
const DEFAULT_RADIUS_KM: f64 = 30.0;

/// Maximum number of members to notify per need
const MAX_NOTIFICATIONS: usize = 5;

/// Matching effect - orchestrates the full matching pipeline
///
/// Pipeline:
/// 1. Get listing + embedding
/// 2. Vector search (distance filtered)
/// 3. AI relevance check (generous threshold)
/// 4. Throttle check (max 3/week)
/// 5. Send notifications (top 5)
/// 6. Track in notifications table
pub struct MatchingEffect;

#[async_trait]
impl Effect<MatchingCommand, ServerDeps> for MatchingEffect {
    type Event = MatchingEvent;

    async fn execute(
        &self,
        cmd: MatchingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<MatchingEvent> {
        match cmd {
            MatchingCommand::FindMatches { listing_id } => {
                info!("Finding matches for need: {}", listing_id);

                // 1. Get listing from database
                let listing = match Listing::find_by_id(listing_id, &ctx.deps().db_pool).await {
                    Ok(need) => need,
                    Err(e) => {
                        error!("Failed to fetch listing {}: {}", listing_id, e);
                        return Ok(MatchingEvent::MatchingFailed {
                            listing_id,
                            error: format!("Need not found: {}", e),
                        });
                    }
                };

                // TODO: Generate embedding if not exists
                // For now, assume embeddings are generated separately
                let need_embedding = match &listing.embedding {
                    Some(emb) => emb.clone(),
                    None => {
                        warn!("Need {} has no embedding, skipping matching", listing_id);
                        return Ok(MatchingEvent::NoMatchesFound {
                            listing_id,
                            reason: "Need has no embedding".to_string(),
                        });
                    }
                };

                // 2. Vector search with distance filtering
                let embedding_slice: &[f32] = need_embedding.as_slice();
                let candidates =
                    if let (Some(lat), Some(lng)) = (listing.latitude, listing.longitude) {
                        // Has location - filter by distance
                        MatchCandidate::find_within_radius(
                            embedding_slice,
                            lat,
                            lng,
                            DEFAULT_RADIUS_KM,
                            &ctx.deps().db_pool,
                        )
                        .await?
                    } else {
                        // No location - search statewide
                        MatchCandidate::find_statewide(embedding_slice, &ctx.deps().db_pool).await?
                    };

                if candidates.is_empty() {
                    info!("No candidates found for listing {}", listing_id);
                    return Ok(MatchingEvent::NoMatchesFound {
                        listing_id,
                        reason: "No candidates in range".to_string(),
                    });
                }

                debug!("Found {} candidates for relevance check", candidates.len());

                // 3. Filter relevant candidates and check throttles
                let mut eligible_notifications = Vec::new();

                for candidate in candidates.iter().take(MAX_NOTIFICATIONS) {
                    // Check relevance using pure utility function
                    let relevance_result = check_relevance_by_similarity(candidate.similarity);

                    if !relevance_result.is_relevant {
                        debug!("Candidate {} not relevant, skipping", candidate.member_id);
                        continue;
                    }

                    // Increment notification count (atomic throttle check)
                    // This must remain sequential to prevent race conditions
                    match Member::increment_notification_count(
                        candidate.member_id,
                        &ctx.deps().db_pool,
                    )
                    .await?
                    {
                        Some(_) => {
                            info!(
                                "Member {} eligible for notification about listing {}",
                                candidate.member_id, listing_id
                            );
                            eligible_notifications
                                .push((candidate.clone(), relevance_result.explanation.clone()));
                        }
                        None => {
                            debug!(
                                "Member {} at notification limit, skipping",
                                candidate.member_id
                            );
                        }
                    }
                }

                if eligible_notifications.is_empty() {
                    info!("No eligible members to notify for listing {}", listing_id);
                    return Ok(MatchingEvent::MatchesFound {
                        listing_id,
                        candidate_count: candidates.len(),
                        notified_count: 0,
                    });
                }

                // 4. Send push notifications concurrently (5x speedup)
                let push_service = ctx.deps().push_service.clone();
                let need_clone = listing.clone();

                let push_results: Vec<_> = stream::iter(eligible_notifications.into_iter())
                    .map(|(candidate, why_relevant)| {
                        let listing = need_clone.clone();
                        let push_service = push_service.clone();
                        let member_id = candidate.member_id;

                        async move {
                            send_push_notification(
                                &candidate,
                                &listing,
                                &why_relevant,
                                push_service.as_ref(),
                            )
                            .await
                            .map(|_| (member_id, why_relevant))
                        }
                    })
                    .buffer_unordered(5) // Send 5 notifications concurrently
                    .collect()
                    .await;

                // 5. Batch insert notification records for successful sends
                let successful_notifications: Vec<_> = push_results
                    .into_iter()
                    .filter_map(|result| result.ok())
                    .collect();

                if !successful_notifications.is_empty() {
                    Notification::batch_create(
                        listing_id,
                        &successful_notifications,
                        &ctx.deps().db_pool,
                    )
                    .await?;
                }

                let notified_count = successful_notifications.len();

                info!(
                    "Matching complete for listing {}: {} candidates, {} notified",
                    listing_id,
                    candidates.len(),
                    notified_count
                );

                Ok(MatchingEvent::MatchesFound {
                    listing_id,
                    candidate_count: candidates.len(),
                    notified_count,
                })
            }
        }
    }
}

// Relevance checking moved to domains/matching/utils/relevance.rs
// This keeps the effect focused on orchestration (I/O) rather than business logic

/// Send push notification via Expo
#[instrument(skip(candidate, listing, push_service), fields(member_id = %candidate.member_id, listing_id = %listing.id, token = %candidate.expo_push_token))]
async fn send_push_notification(
    candidate: &MatchCandidate,
    listing: &Listing,
    why_relevant: &str,
    push_service: &dyn crate::kernel::BasePushNotificationService,
) -> Result<()> {
    let title = "You might be interested in this";
    let body = format!("{} - {}", listing.organization_name, listing.title);

    debug!(title = %title, body = %body, "Sending Expo push notification");

    let data = serde_json::json!({
        "listing_id": listing.id.to_string(),
        "organization": listing.organization_name,
        "why_relevant": why_relevant,
    });

    push_service
        .send_notification(&candidate.expo_push_token, title, &body, data)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to send Expo push notification");
            e
        })?;

    info!("Successfully sent push notification");
    Ok(())
}

/// Record notification in database
#[instrument(skip(pool, why_relevant), fields(listing_id = %listing_id, member_id = %member_id))]
async fn record_notification(
    listing_id: ListingId,
    member_id: MemberId,
    why_relevant: &str,
    pool: &PgPool,
) -> Result<()> {
    debug!("Recording notification in database");

    Notification::record(listing_id, member_id, why_relevant.to_string(), pool)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to record notification");
            e
        })?;

    debug!("Successfully recorded notification");
    Ok(())
}
