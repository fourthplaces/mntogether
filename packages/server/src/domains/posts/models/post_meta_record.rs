use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Meta field group: editorial metadata (kicker, byline, timestamps, deck).
/// 1:1 relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostMetaRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub kicker: Option<String>,
    pub byline: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub updated: Option<String>,
    pub deck: Option<String>,
}

impl PostMetaRecord {
    /// Batch-fetch meta records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_meta WHERE post_id = ANY($1)",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
