use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Media field group: images with caption and credit.
/// 1:many relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostMediaRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub image_url: Option<String>,
    pub caption: Option<String>,
    pub credit: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl PostMediaRecord {
    /// Batch-fetch media records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_media WHERE post_id = ANY($1) ORDER BY post_id, sort_order",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Upsert the primary media record for a post (sort_order = 0).
    /// Updates existing record if found, otherwise inserts.
    pub async fn upsert_primary(
        post_id: Uuid,
        image_url: Option<&str>,
        caption: Option<&str>,
        credit: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        // Try to find existing primary media
        let existing = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_media WHERE post_id = $1 ORDER BY sort_order LIMIT 1",
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?;

        if let Some(record) = existing {
            let row = sqlx::query_as::<_, Self>(
                r#"UPDATE post_media SET image_url = $2, caption = $3, credit = $4
                WHERE id = $1 RETURNING *"#,
            )
            .bind(record.id)
            .bind(image_url)
            .bind(caption)
            .bind(credit)
            .fetch_one(pool)
            .await?;
            Ok(row)
        } else {
            let row = sqlx::query_as::<_, Self>(
                r#"INSERT INTO post_media (post_id, image_url, caption, credit, sort_order)
                VALUES ($1, $2, $3, $4, 0) RETURNING *"#,
            )
            .bind(post_id)
            .bind(image_url)
            .bind(caption)
            .bind(credit)
            .fetch_one(pool)
            .await?;
            Ok(row)
        }
    }
}
