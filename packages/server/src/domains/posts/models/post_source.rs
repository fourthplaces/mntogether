use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{PostId, PostSourceId};

/// A source from which a post was discovered (website, Instagram, etc.)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostSource {
    pub id: PostSourceId,
    pub post_id: PostId,
    pub source_type: String,
    pub source_id: Uuid,
    pub source_url: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub disappeared_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PostSource {
    /// Find all sources for a post
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_sources WHERE post_id = $1 ORDER BY created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new post source link
    pub async fn create(
        post_id: PostId,
        source_type: &str,
        source_id: Uuid,
        source_url: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_sources (post_id, source_type, source_id, source_url)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(source_type)
        .bind(source_id)
        .bind(source_url)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Copy all sources from one post to another (for revision creation)
    pub async fn copy_sources(
        from_post_id: PostId,
        to_post_id: PostId,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO post_sources (post_id, source_type, source_id, source_url,
                first_seen_at, last_seen_at, disappeared_at)
            SELECT $2, source_type, source_id, source_url,
                first_seen_at, last_seen_at, disappeared_at
            FROM post_sources
            WHERE post_id = $1
            ON CONFLICT (post_id, source_type, source_id) DO NOTHING
            "#,
        )
        .bind(from_post_id)
        .bind(to_post_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
