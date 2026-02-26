use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// Profile fields for a person or community member (Spotlight type). 1:1 with post.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostPerson {
    pub id: Uuid,
    pub post_id: PostId,
    pub name: Option<String>,
    pub role: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub quote: Option<String>,
}

impl PostPerson {
    /// Find the person profile for a post, if any.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM post_person WHERE post_id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Upsert the person profile for a post.
    pub async fn upsert(
        post_id: PostId,
        name: Option<&str>,
        role: Option<&str>,
        bio: Option<&str>,
        photo_url: Option<&str>,
        quote: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_person (post_id, name, role, bio, photo_url, quote)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (post_id)
            DO UPDATE SET name = $2, role = $3, bio = $4, photo_url = $5, quote = $6
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(name)
        .bind(role)
        .bind(bio)
        .bind(photo_url)
        .bind(quote)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete the person profile for a post.
    pub async fn delete_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_person WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
