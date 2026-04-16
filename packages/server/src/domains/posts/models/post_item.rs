use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Items field group: name+detail pairs (exchanges, references).
/// 1:many relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostItem {
    pub id: Uuid,
    pub post_id: Uuid,
    pub name: String,
    pub detail: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Input for replacing the items list on a post.
#[derive(Debug, Clone, Deserialize)]
pub struct PostItemInput {
    pub name: String,
    pub detail: Option<String>,
}

impl PostItem {
    /// Batch-fetch items for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_items WHERE post_id = ANY($1) ORDER BY post_id, sort_order",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Replace the entire items list for a post. Deletes existing rows, inserts
    /// the provided list in order.
    pub async fn replace_all(post_id: Uuid, items: &[PostItemInput], pool: &PgPool) -> Result<()> {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM post_items WHERE post_id = $1")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
        for (i, item) in items.iter().enumerate() {
            sqlx::query(
                "INSERT INTO post_items (post_id, name, detail, sort_order) VALUES ($1, $2, $3, $4)",
            )
            .bind(post_id)
            .bind(&item.name)
            .bind(item.detail.as_deref())
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
