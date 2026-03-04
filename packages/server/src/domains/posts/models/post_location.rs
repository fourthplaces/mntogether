use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{LocationId, PostId, PostLocationId};

/// Links posts to locations (HSDS service_at_location equivalent)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostLocation {
    pub id: PostLocationId,
    pub post_id: PostId,
    pub location_id: LocationId,
    pub is_primary: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PostLocation {
    pub async fn create(
        post_id: PostId,
        location_id: LocationId,
        is_primary: bool,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let post_location = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_locations (post_id, location_id, is_primary, notes)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (post_id, location_id) DO UPDATE SET
                is_primary = EXCLUDED.is_primary,
                notes = COALESCE(EXCLUDED.notes, post_locations.notes)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(location_id)
        .bind(is_primary)
        .bind(notes)
        .fetch_one(pool)
        .await?;
        Ok(post_location)
    }

    pub async fn delete(post_id: PostId, location_id: LocationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_locations WHERE post_id = $1 AND location_id = $2")
            .bind(post_id)
            .bind(location_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_locations WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
