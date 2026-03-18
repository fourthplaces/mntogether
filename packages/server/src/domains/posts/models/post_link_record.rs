use anyhow::Result;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Link field group: CTA button with optional deadline.
/// 1:1 relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostLinkRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub label: Option<String>,
    pub url: Option<String>,
    pub deadline: Option<NaiveDate>,
}

impl PostLinkRecord {
    /// Batch-fetch link records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_link WHERE post_id = ANY($1)",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
