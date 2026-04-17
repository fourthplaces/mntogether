use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::media::models::{DesiredRef, MediaReference};

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
    /// FK to media.id when this image came from the Library. NULL when
    /// image_url is an external paste / legacy value.
    pub media_id: Option<Uuid>,
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

    /// Upsert the primary media record for a post (sort_order = 0).
    /// When `media_id` is provided, also reconciles `media_references` for
    /// (`post_hero`, post_id) so the Media Library usage panel stays current.
    pub async fn upsert_primary(
        post_id: Uuid,
        image_url: Option<&str>,
        caption: Option<&str>,
        credit: Option<&str>,
        media_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        // Try to find existing primary media
        let existing = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_media WHERE post_id = $1 ORDER BY sort_order LIMIT 1",
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?;

        let row = if let Some(record) = existing {
            sqlx::query_as::<_, Self>(
                r#"UPDATE post_media
                   SET image_url = $2, caption = $3, credit = $4, media_id = $5
                   WHERE id = $1 RETURNING *"#,
            )
            .bind(record.id)
            .bind(image_url)
            .bind(caption)
            .bind(credit)
            .bind(media_id)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_as::<_, Self>(
                r#"INSERT INTO post_media (post_id, image_url, caption, credit, sort_order, media_id)
                   VALUES ($1, $2, $3, $4, 0, $5) RETURNING *"#,
            )
            .bind(post_id)
            .bind(image_url)
            .bind(caption)
            .bind(credit)
            .bind(media_id)
            .fetch_one(pool)
            .await?
        };

        // Reconcile media_references for post_hero — either the new
        // `media_id` becomes the only ref, or the ref is cleared (when
        // media_id is None, e.g. the editor pasted a raw URL).
        let desired: Vec<DesiredRef> = match media_id {
            Some(mid) => vec![DesiredRef { media_id: mid, field_key: None }],
            None => Vec::new(),
        };
        MediaReference::reconcile("post_hero", post_id, &desired, pool).await?;

        Ok(row)
    }
}
