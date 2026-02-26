use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// Editorial metadata for a post — kicker, byline, timestamps (1:1 with post).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostMeta {
    pub id: Uuid,
    pub post_id: PostId,
    pub kicker: Option<String>,
    pub byline: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub updated: Option<String>,
}

impl PostMeta {
    /// Find the meta for a post, if any.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM post_meta WHERE post_id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Upsert the meta for a post.
    pub async fn upsert(
        post_id: PostId,
        kicker: Option<&str>,
        byline: Option<&str>,
        timestamp: Option<DateTime<Utc>>,
        updated: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_meta (post_id, kicker, byline, timestamp, updated)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (post_id)
            DO UPDATE SET kicker = $2, byline = $3, timestamp = $4, updated = $5
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(kicker)
        .bind(byline)
        .bind(timestamp)
        .bind(updated)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete the meta for a post.
    pub async fn delete_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_meta WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
