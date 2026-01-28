pub mod vector_search;

use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use sqlx::PgPool;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::domains::matching::{commands::MatchingCommand, events::MatchingEvent};
use crate::domains::member::models::member::Member;
use crate::domains::organization::effects::ServerDeps;
use crate::domains::organization::models::need::OrganizationNeed;

use vector_search::{find_members_statewide, find_members_within_radius, MatchCandidate};

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
                let candidates = if let (Some(lat), Some(lng)) = (need.latitude, need.longitude) {
                    // Has location - filter by distance
                    find_members_within_radius(
                        &need_embedding,
                        lat,
                        lng,
                        DEFAULT_RADIUS_KM,
                        &ctx.deps().db_pool,
                    )
                    .await?
                } else {
                    // No location - search statewide
                    find_members_statewide(&need_embedding, &ctx.deps().db_pool).await?
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
                    // Check relevance (placeholder - would use GPT-4 in production)
                    let (is_relevant, why_relevant) = check_relevance(&need, &candidate).await?;

                    if !is_relevant {
                        debug!("Candidate {} not relevant, skipping", candidate.member_id);
                        continue;
                    }

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
                                &ctx.deps().expo_client,
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

/// Check if a candidate is relevant for a need (AI relevance check)
///
/// Uses GPT-4o-mini to evaluate if the member's skills/interests match the need
#[instrument(skip(need, candidate), fields(need_id = %need.id, member_id = %candidate.member_id, similarity = %candidate.similarity))]
async fn check_relevance(
    need: &OrganizationNeed,
    candidate: &MatchCandidate,
) -> Result<(bool, String)> {
    // Quick pre-filter: if similarity is very low, skip AI call
    if candidate.similarity < 0.4 {
        debug!("Low similarity score, rejecting without AI call");
        return Ok((false, "Low similarity score".to_string()));
    }

    // For high similarity, trust the embedding
    if candidate.similarity > 0.8 {
        info!("High similarity score, accepting without AI call");
        return Ok((
            true,
            format!(
                "Strong match based on your interests and skills ({}% similar)",
                (candidate.similarity * 100.0) as i32
            ),
        ));
    }

    // For medium similarity, use AI to decide
    // Note: This is commented out to avoid excessive API costs in development
    // Uncomment for production or when you have sufficient API quota

    /*
        use reqwest::Client;

        let client = Client::new();
        let api_key = std::env::var("OPENAI_API_KEY")?;

        let prompt = format!(
            r#"Evaluate if this volunteer is a good match for this opportunity.

    Organization Need:
    {}

    Volunteer Profile:
    {}

    Respond with ONLY:
    - "YES|" followed by a one-sentence explanation if they're a good match
    - "NO|Not a good match" if they're not

    Be generous - if there's ANY reasonable connection, say YES."#,
            need.description,
            candidate.searchable_text
        );

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": "gpt-4o-mini",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a volunteer matching assistant. Be generous with matches."
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.3,
                "max_tokens": 100
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("NO|Error");

        let parts: Vec<&str> = content.splitn(2, '|').collect();
        let is_relevant = parts[0].trim() == "YES";
        let explanation = parts.get(1).unwrap_or(&"").trim().to_string();

        Ok((is_relevant, explanation))
        */

    // Fallback: use similarity threshold
    let is_relevant = candidate.similarity > 0.6;
    let why_relevant = if is_relevant {
        format!(
            "Your profile matches this opportunity ({}% similar)",
            (candidate.similarity * 100.0) as i32
        )
    } else {
        "Not a strong match".to_string()
    };

    Ok((is_relevant, why_relevant))
}

/// Send push notification via Expo
#[instrument(skip(candidate, need, expo_client), fields(member_id = %candidate.member_id, need_id = %need.id, token = %candidate.expo_push_token))]
async fn send_push_notification(
    candidate: &MatchCandidate,
    need: &OrganizationNeed,
    why_relevant: &str,
    expo_client: &crate::common::utils::ExpoClient,
) -> Result<()> {
    let title = "You might be interested in this";
    let body = format!("{} - {}", need.organization_name, need.title);

    debug!(title = %title, body = %body, "Sending Expo push notification");

    let data = serde_json::json!({
        "need_id": need.id.to_string(),
        "organization": need.organization_name,
        "why_relevant": why_relevant,
    });

    expo_client
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
    need_id: Uuid,
    member_id: Uuid,
    why_relevant: &str,
    pool: &PgPool,
) -> Result<()> {
    debug!("Recording notification in database");

    sqlx::query(
        "INSERT INTO notifications (need_id, member_id, why_relevant)
         VALUES ($1, $2, $3)
         ON CONFLICT (need_id, member_id) DO NOTHING",
    )
    .bind(need_id)
    .bind(member_id)
    .bind(why_relevant)
    .execute(pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to record notification");
        e
    })?;

    debug!("Successfully recorded notification");
    Ok(())
}
