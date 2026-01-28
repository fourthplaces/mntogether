use anyhow::Result;
use sqlx::PgPool;
use tracing::{debug, info, instrument};
use uuid::Uuid;

/// Candidate member for matching (from vector search)
#[derive(Debug, Clone)]
pub struct MatchCandidate {
    pub member_id: Uuid,
    pub expo_push_token: String,
    pub searchable_text: String,
    pub similarity: f64,
    pub distance_km: Option<f64>,
}

/// Find members matching a need using distance filtering + vector similarity
///
/// Strategy:
/// 1. Filter by distance (30km radius)
/// 2. Rank by embedding similarity
/// 3. Return top 20 candidates for AI relevance check
///
/// # Arguments
/// * `need_embedding` - Vector embedding of the need
/// * `org_lat`, `org_lng` - Organization coordinates
/// * `radius_km` - Search radius (default 30km)
/// * `pool` - Database connection pool
#[instrument(skip(need_embedding, pool), fields(lat = %org_lat, lng = %org_lng, radius_km = %radius_km))]
pub async fn find_members_within_radius(
    need_embedding: &[f32],
    org_lat: f64,
    org_lng: f64,
    radius_km: f64,
    pool: &PgPool,
) -> Result<Vec<MatchCandidate>> {
    debug!(
        "Searching for members within {}km of ({}, {})",
        radius_km, org_lat, org_lng
    );

    // TODO: Add embedding column to members table
    // For now, this query assumes embeddings exist
    // In production, you'd generate embeddings in a background job

    let candidates = sqlx::query_as::<_, (Uuid, String, String, f64, f64)>(
        "SELECT
            m.id,
            m.expo_push_token,
            m.searchable_text,
            1 - (m.embedding <=> $1) AS similarity,
            haversine_distance($2, $3, m.latitude, m.longitude) AS distance_km
         FROM members m
         WHERE m.active = true
           AND m.latitude IS NOT NULL
           AND m.longitude IS NOT NULL
           AND m.embedding IS NOT NULL
           AND m.notification_count_this_week < 3
           AND haversine_distance($2, $3, m.latitude, m.longitude) <= $4
         ORDER BY similarity DESC
         LIMIT 20"
    )
    .bind(need_embedding)
    .bind(org_lat)
    .bind(org_lng)
    .bind(radius_km)
    .fetch_all(pool)
    .await?;

    let results: Vec<MatchCandidate> = candidates
        .into_iter()
        .map(|(id, token, text, similarity, distance)| MatchCandidate {
            member_id: id,
            expo_push_token: token,
            searchable_text: text,
            similarity,
            distance_km: Some(distance),
        })
        .collect();

    info!("Found {} candidates within {}km", results.len(), radius_km);

    Ok(results)
}

/// Fallback: Find members statewide (no location filter)
///
/// Used when organization has no location data
#[instrument(skip(need_embedding, pool))]
pub async fn find_members_statewide(
    need_embedding: &[f32],
    pool: &PgPool,
) -> Result<Vec<MatchCandidate>> {
    debug!("Searching for members statewide (no location filter)");

    let candidates = sqlx::query_as::<_, (Uuid, String, String, f64)>(
        "SELECT
            m.id,
            m.expo_push_token,
            m.searchable_text,
            1 - (m.embedding <=> $1) AS similarity
         FROM members m
         WHERE m.active = true
           AND m.embedding IS NOT NULL
           AND m.notification_count_this_week < 3
         ORDER BY similarity DESC
         LIMIT 20"
    )
    .bind(need_embedding)
    .fetch_all(pool)
    .await?;

    let results: Vec<MatchCandidate> = candidates
        .into_iter()
        .map(|(id, token, text, similarity)| MatchCandidate {
            member_id: id,
            expo_push_token: token,
            searchable_text: text,
            similarity,
            distance_km: None,
        })
        .collect();

    info!("Found {} candidates statewide", results.len());

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_candidate() {
        let candidate = MatchCandidate {
            member_id: Uuid::new_v4(),
            expo_push_token: "token".to_string(),
            searchable_text: "Can drive".to_string(),
            similarity: 0.85,
            distance_km: Some(15.5),
        };

        assert!(candidate.similarity > 0.8);
        assert!(candidate.distance_km.unwrap() < 20.0);
    }
}
