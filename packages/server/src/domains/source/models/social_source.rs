use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{SocialSourceId, SourceId};

/// Social-media-specific source details (1:1 extension of sources)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SocialSource {
    pub id: SocialSourceId,
    pub source_id: SourceId,
    pub source_type: String, // denormalized for UNIQUE constraint
    pub handle: String,
}

impl SocialSource {
    pub async fn find_by_source_id(source_id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM social_sources WHERE source_id = $1")
            .bind(source_id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_source_id_optional(
        source_id: SourceId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM social_sources WHERE source_id = $1")
            .bind(source_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_handle(
        source_type: &str,
        handle: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM social_sources WHERE source_type = $1 AND handle = $2",
        )
        .bind(source_type)
        .bind(handle)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(
        source_id: SourceId,
        source_type: &str,
        handle: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO social_sources (source_id, source_type, handle)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(source_id)
        .bind(source_type)
        .bind(handle)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
