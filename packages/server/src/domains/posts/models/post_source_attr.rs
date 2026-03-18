use anyhow::Result;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Source attribution field group: who issued this content.
/// 1:1 relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostSourceAttr {
    pub id: Uuid,
    pub post_id: Uuid,
    pub source_name: Option<String>,
    pub attribution: Option<String>,
}

impl PostSourceAttr {
    /// Batch-fetch source attribution records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_source_attribution WHERE post_id = ANY($1)",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
