use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{MemberId, PaginationDirection, Readable, ValidatedPaginationArgs};

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
    ///
    /// IMPORTANT: This uses SELECT FOR UPDATE to prevent race conditions where
    /// concurrent transactions could both see count=2 and both increment to 3+.
    /// The row lock ensures atomic check-and-increment.
    pub async fn increment_notification_count(id: MemberId, pool: &PgPool) -> Result<Option<Self>> {
        // Start transaction for atomic check-and-increment
        let mut tx = pool.begin().await?;

        // Lock the row with SELECT FOR UPDATE to prevent concurrent modifications
        let current: Option<Self> =
            sqlx::query_as("SELECT * FROM members WHERE id = $1 FOR UPDATE")
                .bind(id.into_uuid())
                .fetch_optional(&mut *tx)
                .await?;

        // Check if member exists
        let current = match current {
            Some(m) => m,
            None => {
                tx.rollback().await?;
                return Ok(None);
            }
        };

        // Check throttle limit
        if current.notification_count_this_week >= 3 {
            tx.rollback().await?;
            return Ok(None);
        }

        // Increment count (row is locked, safe to increment)
        let updated: Self = sqlx::query_as(
            "UPDATE members
             SET notification_count_this_week = notification_count_this_week + 1
             WHERE id = $1
             RETURNING *",
        )
        .bind(id.into_uuid())
        .fetch_one(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        Ok(Some(updated))
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

    /// Count all members
    pub async fn count(pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM members")
            .fetch_one(pool)
            .await?;
        Ok(count)
    }

    /// Find members with cursor-based pagination (Relay spec)
    pub async fn find_paginated(
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM members WHERE ($1::uuid IS NULL OR id > $1) ORDER BY id ASC LIMIT $2",
                )
                .bind(args.cursor)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                let mut rows = sqlx::query_as::<_, Self>(
                    "SELECT * FROM members WHERE ($1::uuid IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
                )
                .bind(args.cursor)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await?;
                rows.reverse();
                rows
            }
        };

        let has_more = results.len() > args.limit as usize;
        let results = results.into_iter().take(args.limit as usize).collect();
        Ok((results, has_more))
    }
}

/// Implement Readable for deferred database reads via ReadResult<Member>
#[async_trait]
impl Readable for Member {
    type Id = Uuid;

    async fn read_by_id(id: Self::Id, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM members WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
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
