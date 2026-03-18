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

    /// Upsert person record for a post (1:1, post_id is UNIQUE).
    pub async fn upsert(
        post_id: Uuid,
        name: Option<&str>,
        role: Option<&str>,
        bio: Option<&str>,
        photo_url: Option<&str>,
        quote: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_person (post_id, name, role, bio, photo_url, quote)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (post_id) DO UPDATE SET
                name = EXCLUDED.name,
                role = EXCLUDED.role,
                bio = EXCLUDED.bio,
                photo_url = EXCLUDED.photo_url,
                quote = EXCLUDED.quote
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
        .await?;
        Ok(row)
    }
}
