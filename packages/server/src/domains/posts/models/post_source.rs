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

    /// Find all post-source links for a given source
    pub async fn find_by_source(
        source_type: &str,
        source_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_sources WHERE source_type = $1 AND source_id = $2 ORDER BY created_at ASC",
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find post IDs for a given source (convenience method)
    pub async fn find_post_ids_by_source(
        source_type: &str,
        source_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<PostId>> {
        sqlx::query_scalar::<_, PostId>(
            "SELECT post_id FROM post_sources WHERE source_type = $1 AND source_id = $2",
        )
        .bind(source_type)
        .bind(source_id)
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

    /// Upsert: insert or update last_seen_at on conflict
    pub async fn upsert(
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
            ON CONFLICT (post_id, source_type, source_id)
            DO UPDATE SET last_seen_at = NOW(), updated_at = NOW(),
                          source_url = COALESCE($4, post_sources.source_url)
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

    /// Mark a source link as disappeared
    pub async fn mark_disappeared(
        post_id: PostId,
        source_type: &str,
        source_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE post_sources
            SET disappeared_at = NOW(), updated_at = NOW()
            WHERE post_id = $1 AND source_type = $2 AND source_id = $3
              AND disappeared_at IS NULL
            "#,
        )
        .bind(post_id)
        .bind(source_type)
        .bind(source_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Count posts grouped by source_id for a set of source IDs of a given type
    pub async fn count_by_sources(
        source_type: &str,
        source_ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<Uuid, i64>> {
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT ps.source_id, COUNT(DISTINCT ps.post_id) as count
            FROM post_sources ps
            JOIN posts p ON p.id = ps.post_id
            WHERE ps.source_type = $1
              AND ps.source_id = ANY($2)
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
            GROUP BY ps.source_id
            "#,
        )
        .bind(source_type)
        .bind(source_ids)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Count posts by agent grouped by source for a given source type
    pub async fn count_by_agent_grouped_by_source(
        agent_id: Uuid,
        source_type: &str,
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<Uuid, i64>> {
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT ps.source_id, COUNT(*) as count
            FROM posts p
            JOIN agents a ON a.member_id = p.submitted_by_id
            JOIN post_sources ps ON ps.post_id = p.id
            WHERE a.id = $1
              AND ps.source_type = $2
              AND p.deleted_at IS NULL
            GROUP BY ps.source_id
            "#,
        )
        .bind(agent_id)
        .bind(source_type)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Count posts grouped by source_id across ANY source_type
    pub async fn count_by_sources_any_type(
        source_ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<Uuid, i64>> {
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT ps.source_id, COUNT(DISTINCT ps.post_id) as count
            FROM post_sources ps
            JOIN posts p ON p.id = ps.post_id
            WHERE ps.source_id = ANY($1)
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
            GROUP BY ps.source_id
            "#,
        )
        .bind(source_ids)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Move all source links from one post to another (for merge/dedup).
    /// Uses INSERT ... SELECT with ON CONFLICT to update last_seen_at if already linked.
    pub async fn move_to_post(
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
            ON CONFLICT (post_id, source_type, source_id)
            DO UPDATE SET last_seen_at = GREATEST(post_sources.last_seen_at, EXCLUDED.last_seen_at),
                          updated_at = NOW()
            "#,
        )
        .bind(from_post_id)
        .bind(to_post_id)
        .execute(pool)
        .await?;
        Ok(())
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
