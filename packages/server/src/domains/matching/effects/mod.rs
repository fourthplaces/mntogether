pub mod vector_search;

use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use sqlx::PgPool;
use tracing::{debug, error, info, instrument, warn};

use crate::common::{MemberId, NeedId};
use crate::domains::matching::{
    commands::MatchingCommand, events::MatchingEvent, models::notification::Notification,
    utils::check_relevance_by_similarity,
};
use crate::domains::member::models::member::Member;
use crate::domains::organization::effects::ServerDeps;
use crate::domains::organization::models::need::OrganizationNeed;

use vector_search::MatchCandidate;

/// Default search radius in kilometers (≈ 20 miles)
const DEFAULT_RADIUS_KM: f64 = 30.0;

/// Maximum number of members to notify per need
const MAX_NOTIFICATIONS: usize = 5;

/// Matching effect - orchestrates the full matching pipeline
///
/// Pipeline:
/// 1. Get need + embedding
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
            MatchingCommand::FindMatches { need_id } => {
                info!("Finding matches for need: {}", need_id);

                // 1. Get need from database
                let need = match OrganizationNeed::find_by_id(need_id, &ctx.deps().db_pool).await {
                    Ok(need) => need,
                    Err(e) => {
                        error!("Failed to fetch need {}: {}", need_id, e);
                        return Ok(MatchingEvent::MatchingFailed {
                            need_id,
                            error: format!("Need not found: {}", e),
                        });
                    }
                };

                // TODO: Generate embedding if not exists
                // For now, assume embeddings are generated separately
                let need_embedding = match &need.embedding {
                    Some(emb) => emb.clone(),
                    None => {
                        warn!("Need {} has no embedding, skipping matching", need_id);
                        return Ok(MatchingEvent::NoMatchesFound {
                            need_id,
                            reason: "Need has no embedding".to_string(),
                        });
                    }
                };

                // 2. Vector search with distance filtering
                let embedding_slice: &[f32] = need_embedding.as_slice();
                let candidates = if let (Some(lat), Some(lng)) = (need.latitude, need.longitude) {
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
                    info!("No candidates found for need {}", need_id);
                    return Ok(MatchingEvent::NoMatchesFound {
                        need_id,
                        reason: "No candidates in range".to_string(),
                    });
                }

                debug!("Found {} candidates for relevance check", candidates.len());

                // 3. AI relevance check + send notifications
                let mut notified_count = 0;
                for candidate in candidates.iter().take(MAX_NOTIFICATIONS) {
                    // Check relevance using pure utility function
                    let relevance_result = check_relevance_by_similarity(candidate.similarity);

                    if !relevance_result.is_relevant {
                        debug!("Candidate {} not relevant, skipping", candidate.member_id);
                        continue;
                    }

                    let why_relevant = relevance_result.explanation;

                    // Increment notification count (atomic throttle check)
                    match Member::increment_notification_count(
                        candidate.member_id,
                        &ctx.deps().db_pool,
                    )
                    .await?
                    {
                        Some(_) => {
                            // Successfully incremented - send notification
                            info!(
                                "Notifying member {} about need {}",
                                candidate.member_id, need_id
                            );

                            // Send Expo push notification
                            send_push_notification(
                                &candidate,
                                &need,
                                &why_relevant,
                                ctx.deps().push_service.as_ref(),
                            )
                            .await?;

                            // Track notification in database
                            record_notification(
                                need_id,
                                candidate.member_id,
                                &why_relevant,
                                &ctx.deps().db_pool,
                            )
                            .await?;

                            notified_count += 1;
                        }
                        None => {
                            debug!(
                                "Member {} at notification limit, skipping",
                                candidate.member_id
                            );
                        }
                    }
                }

                info!(
                    "Matching complete for need {}: {} candidates, {} notified",
                    need_id,
                    candidates.len(),
                    notified_count
                );

                Ok(MatchingEvent::MatchesFound {
                    need_id,
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
#[instrument(skip(candidate, need, push_service), fields(member_id = %candidate.member_id, need_id = %need.id, token = %candidate.expo_push_token))]
async fn send_push_notification(
    candidate: &MatchCandidate,
    need: &OrganizationNeed,
    why_relevant: &str,
    push_service: &dyn crate::kernel::BasePushNotificationService,
) -> Result<()> {
    let title = "You might be interested in this";
    let body = format!("{} - {}", need.organization_name, need.title);

    debug!(title = %title, body = %body, "Sending Expo push notification");

    let data = serde_json::json!({
        "need_id": need.id.to_string(),
        "organization": need.organization_name,
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
#[instrument(skip(pool, why_relevant), fields(need_id = %need_id, member_id = %member_id))]
async fn record_notification(
    need_id: NeedId,
    member_id: MemberId,
    why_relevant: &str,
    pool: &PgPool,
) -> Result<()> {
    debug!("Recording notification in database");

    Notification::record(need_id, member_id, why_relevant.to_string(), pool)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to record notification");
            e
        })?;

    debug!("Successfully recorded notification");
    Ok(())
}
