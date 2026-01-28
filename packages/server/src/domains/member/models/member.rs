use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Member model - SQL persistence layer
///
/// Privacy-first: No PII, only expo_push_token for anonymous notifications
/// Text-first: searchable_text is source of truth for all capabilities/skills
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Member {
    pub id: Uuid,
    pub expo_push_token: String,
    pub searchable_text: String,

    // Location (coarse precision for matching)
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_name: Option<String>, // "Minneapolis, MN"

    // Status
    pub active: bool,
    pub notification_count_this_week: i32,
    pub paused_until: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
}

impl Member {
    /// Find member by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM members WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Find member by expo push token
    pub async fn find_by_token(token: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM members WHERE expo_push_token = $1")
            .bind(token)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find all active members
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM members WHERE active = true ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find active members within radius of a location (for matching)
    ///
    /// Uses Haversine distance function. Returns members sorted by vector similarity.
    pub async fn find_within_radius(
        center_lat: f64,
        center_lng: f64,
        radius_km: f64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT m.*
             FROM members m
             WHERE m.active = true
               AND m.latitude IS NOT NULL
               AND m.longitude IS NOT NULL
               AND haversine_distance($1, $2, m.latitude, m.longitude) <= $3
             ORDER BY m.created_at DESC",
        )
        .bind(center_lat)
        .bind(center_lng)
        .bind(radius_km)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Insert new member
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO members (
                expo_push_token,
                searchable_text,
                latitude,
                longitude,
                location_name,
                active,
                notification_count_this_week
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(&self.expo_push_token)
        .bind(&self.searchable_text)
        .bind(self.latitude)
        .bind(self.longitude)
        .bind(&self.location_name)
        .bind(self.active)
        .bind(self.notification_count_this_week)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update member status
    pub async fn update_status(id: Uuid, active: bool, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("UPDATE members SET active = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(active)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Increment notification count (for throttling)
    ///
    /// Returns the updated member if successful (count < 3), None if limit reached
    pub async fn increment_notification_count(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "UPDATE members
             SET notification_count_this_week = notification_count_this_week + 1
             WHERE id = $1
               AND notification_count_this_week < 3
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Reset weekly notification counts (called by weekly cron job)
    pub async fn reset_weekly_counts(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query("UPDATE members SET notification_count_this_week = 0")
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Update member embedding
    pub async fn update_embedding(id: Uuid, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE members SET embedding = $2 WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_struct() {
        // Just verify struct compiles
        let member = Member {
            id: Uuid::new_v4(),
            expo_push_token: "ExponentPushToken[xyz]".to_string(),
            searchable_text: "Can drive, Spanish speaker".to_string(),
            latitude: Some(44.98),
            longitude: Some(-93.27),
            location_name: Some("Minneapolis, MN".to_string()),
            active: true,
            notification_count_this_week: 0,
            paused_until: None,
            created_at: Utc::now(),
        };

        assert_eq!(member.active, true);
    }
}
