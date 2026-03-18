use anyhow::Result;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Status field group: exchange state and verification tracking.
/// 1:1 relationship with posts.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostStatusRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub state: Option<String>,
    pub verified: Option<String>,
}

impl PostStatusRecord {
    /// Batch-fetch status records for multiple posts in a single query.
    pub async fn find_by_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_status WHERE post_id = ANY($1)",
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Upsert status record for a post (1:1, post_id is UNIQUE).
    pub async fn upsert(
        post_id: Uuid,
        state: Option<&str>,
        verified: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_status (post_id, state, verified)
            VALUES ($1, $2, $3)
            ON CONFLICT (post_id) DO UPDATE SET
                state = EXCLUDED.state,
                verified = EXCLUDED.verified
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(state)
        .bind(verified)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }
}
