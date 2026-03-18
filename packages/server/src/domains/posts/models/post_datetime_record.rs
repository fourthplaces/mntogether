use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Datetime field group: event timing with optional cost and recurrence.
/// 1:1 relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostDatetimeRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub cost: Option<String>,
    pub recurring: bool,
}

impl PostDatetimeRecord {
    /// Batch-fetch datetime records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_datetime WHERE post_id = ANY($1)",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Upsert datetime record for a post (1:1, post_id is UNIQUE).
    pub async fn upsert(
        post_id: Uuid,
        start_at: Option<DateTime<Utc>>,
        end_at: Option<DateTime<Utc>>,
        cost: Option<&str>,
        recurring: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_datetime (post_id, start_at, end_at, cost, recurring)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (post_id) DO UPDATE SET
                start_at = EXCLUDED.start_at,
                end_at = EXCLUDED.end_at,
                cost = EXCLUDED.cost,
                recurring = EXCLUDED.recurring
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(start_at)
        .bind(end_at)
        .bind(cost)
        .bind(recurring)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }
}
