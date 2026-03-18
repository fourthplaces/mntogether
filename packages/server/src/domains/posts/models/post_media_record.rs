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
}
