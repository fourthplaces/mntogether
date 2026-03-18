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

    /// Upsert meta record for a post (1:1, post_id is UNIQUE).
    pub async fn upsert(
        post_id: Uuid,
        kicker: Option<&str>,
        byline: Option<&str>,
        deck: Option<&str>,
        updated: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_meta (post_id, kicker, byline, deck, updated)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (post_id) DO UPDATE SET
                kicker = EXCLUDED.kicker,
                byline = EXCLUDED.byline,
                deck = EXCLUDED.deck,
                updated = EXCLUDED.updated
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(kicker)
        .bind(byline)
        .bind(deck)
        .bind(updated)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }
}
