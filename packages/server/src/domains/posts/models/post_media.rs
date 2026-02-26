use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// An image with caption and credit belonging to a post.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostMedia {
    pub id: Uuid,
    pub post_id: PostId,
    pub image_url: Option<String>,
    pub caption: Option<String>,
    pub credit: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl PostMedia {
    /// Find all media for a post, ordered by sort_order.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_media WHERE post_id = $1 ORDER BY sort_order ASC, created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new media entry.
    pub async fn create(
        post_id: PostId,
        image_url: Option<&str>,
        caption: Option<&str>,
        credit: Option<&str>,
        sort_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_media (post_id, image_url, caption, credit, sort_order)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(image_url)
        .bind(caption)
        .bind(credit)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete all media for a post.
    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_media WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
