use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::ScheduleId;

/// Operating hours for posts, locations, or post_locations (polymorphic)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Schedule {
    pub id: ScheduleId,
    pub schedulable_type: String, // 'post', 'location', 'post_location'
    pub schedulable_id: Uuid,
    pub day_of_week: i32, // 0=Sunday through 6=Saturday
    pub opens_at: Option<NaiveTime>,
    pub closes_at: Option<NaiveTime>,
    pub timezone: String,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Schedule {
    pub async fn find_by_id(id: ScheduleId, pool: &PgPool) -> Result<Self> {
        let schedule = sqlx::query_as::<_, Self>("SELECT * FROM schedules WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(schedule)
    }

    /// Find all schedules for an entity (post, location, or post_location)
    pub async fn find_for_entity(
        schedulable_type: &str,
        schedulable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let schedules = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM schedules
            WHERE schedulable_type = $1 AND schedulable_id = $2
            ORDER BY day_of_week ASC, opens_at ASC
            "#,
        )
        .bind(schedulable_type)
        .bind(schedulable_id)
        .fetch_all(pool)
        .await?;
        Ok(schedules)
    }

    pub async fn create(
        schedulable_type: &str,
        schedulable_id: Uuid,
        day_of_week: i32,
        opens_at: Option<NaiveTime>,
        closes_at: Option<NaiveTime>,
        timezone: &str,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let schedule = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO schedules (schedulable_type, schedulable_id, day_of_week, opens_at, closes_at, timezone, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(schedulable_type)
        .bind(schedulable_id)
        .bind(day_of_week)
        .bind(opens_at)
        .bind(closes_at)
        .bind(timezone)
        .bind(notes)
        .fetch_one(pool)
        .await?;
        Ok(schedule)
    }

    pub async fn delete(id: ScheduleId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM schedules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all schedules for an entity
    pub async fn delete_all_for_entity(
        schedulable_type: &str,
        schedulable_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query("DELETE FROM schedules WHERE schedulable_type = $1 AND schedulable_id = $2")
            .bind(schedulable_type)
            .bind(schedulable_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
