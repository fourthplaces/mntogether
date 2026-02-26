use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// Source attribution for a post — who issued this notice/content (1:1 with post).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostSourceAttribution {
    pub id: Uuid,
    pub post_id: PostId,
    pub source_name: Option<String>,
    pub attribution: Option<String>,
}

impl PostSourceAttribution {
    /// Find the source attribution for a post, if any.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM post_source_attribution WHERE post_id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Upsert the source attribution for a post.
    pub async fn upsert(
        post_id: PostId,
        source_name: Option<&str>,
        attribution: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_source_attribution (post_id, source_name, attribution)
            VALUES ($1, $2, $3)
            ON CONFLICT (post_id)
            DO UPDATE SET source_name = $2, attribution = $3
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(source_name)
        .bind(attribution)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete the source attribution for a post.
    pub async fn delete_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_source_attribution WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
