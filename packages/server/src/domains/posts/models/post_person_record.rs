use anyhow::Result;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::media::models::{DesiredRef, MediaReference};

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
    /// FK to media.id when the person photo came from the Library.
    pub photo_media_id: Option<Uuid>,
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
    /// Also reconciles `media_references` for (`post_person`, post_id).
    pub async fn upsert(
        post_id: Uuid,
        name: Option<&str>,
        role: Option<&str>,
        bio: Option<&str>,
        photo_url: Option<&str>,
        quote: Option<&str>,
        photo_media_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_person (post_id, name, role, bio, photo_url, quote, photo_media_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (post_id) DO UPDATE SET
                name = EXCLUDED.name,
                role = EXCLUDED.role,
                bio = EXCLUDED.bio,
                photo_url = EXCLUDED.photo_url,
                quote = EXCLUDED.quote,
                photo_media_id = EXCLUDED.photo_media_id
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(name)
        .bind(role)
        .bind(bio)
        .bind(photo_url)
        .bind(quote)
        .bind(photo_media_id)
        .fetch_one(pool)
        .await?;

        let desired: Vec<DesiredRef> = match photo_media_id {
            Some(mid) => vec![DesiredRef { media_id: mid, field_key: None }],
            None => Vec::new(),
        };
        MediaReference::reconcile("post_person", post_id, &desired, pool).await?;

        Ok(row)
    }
}
