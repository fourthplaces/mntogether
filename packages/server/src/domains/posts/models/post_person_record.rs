use anyhow::Result;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Person field group: profile fields for spotlights.
/// 1:1 relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostPersonRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub name: Option<String>,
    pub role: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub quote: Option<String>,
}

impl PostPersonRecord {
    /// Batch-fetch person records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_person WHERE post_id = ANY($1)",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
