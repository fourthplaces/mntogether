use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// CTA button with optional deadline (1:1 with post).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostLink {
    pub id: Uuid,
    pub post_id: PostId,
    pub label: Option<String>,
    pub url: Option<String>,
    pub deadline: Option<NaiveDate>,
}

impl PostLink {
    /// Find the link for a post, if any.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM post_link WHERE post_id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Upsert the link for a post.
    pub async fn upsert(
        post_id: PostId,
        label: Option<&str>,
        url: Option<&str>,
        deadline: Option<NaiveDate>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_link (post_id, label, url, deadline)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (post_id)
            DO UPDATE SET label = $2, url = $3, deadline = $4
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(label)
        .bind(url)
        .bind(deadline)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete the link for a post.
    pub async fn delete_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_link WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
