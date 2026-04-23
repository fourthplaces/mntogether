use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Schedule field group: hours/schedule entries (day + open + close).
/// 1:many relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostScheduleEntry {
    pub id: Uuid,
    pub post_id: Uuid,
    pub day: String,
    pub opens: String,
    pub closes: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Input to replace_all — single `day / opens / closes` triple.
#[derive(Debug, Clone)]
pub struct PostScheduleInput {
    pub day: String,
    pub opens: String,
    pub closes: String,
}

impl PostScheduleEntry {
    /// Batch-fetch schedule entries for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_schedule WHERE post_id = ANY($1) ORDER BY post_id, sort_order",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Replace the schedule block for a post: wipe every row, re-insert the
    /// new set in submission order. Mirrors `PostItem::replace_all`.
    pub async fn replace_all(
        post_id: Uuid,
        entries: &[PostScheduleInput],
        pool: &PgPool,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM post_schedule WHERE post_id = $1")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;

        for (idx, e) in entries.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO post_schedule (post_id, day, opens, closes, sort_order)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(post_id)
            .bind(&e.day)
            .bind(&e.opens)
            .bind(&e.closes)
            .bind(idx as i32)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
