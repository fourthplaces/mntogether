use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// A name+detail pair belonging to a post (Exchange needs/offers, Reference directories).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostItem {
    pub id: Uuid,
    pub post_id: PostId,
    pub name: String,
    pub detail: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl PostItem {
    /// Find all items for a post, ordered by sort_order.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_items WHERE post_id = $1 ORDER BY sort_order ASC, created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new item.
    pub async fn create(
        post_id: PostId,
        name: &str,
        detail: Option<&str>,
        sort_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_items (post_id, name, detail, sort_order)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(name)
        .bind(detail)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Replace all items for a post (delete + bulk insert).
    pub async fn replace_all(
        post_id: PostId,
        items: &[(String, Option<String>)],
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query("DELETE FROM post_items WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;

        let mut results = Vec::with_capacity(items.len());
        for (i, (name, detail)) in items.iter().enumerate() {
            let item = Self::create(post_id, name, detail.as_deref(), i as i32, pool).await?;
            results.push(item);
        }
        Ok(results)
    }

    /// Delete a single item.
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_items WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all items for a post.
    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_items WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
