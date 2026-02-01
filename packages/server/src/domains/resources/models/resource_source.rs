//! ResourceSource model - tracks which pages a resource was extracted from
//!
//! Resources can be extracted from multiple pages (e.g., a service mentioned on
//! both the homepage and a dedicated services page). This model tracks all source
//! pages for each resource.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ResourceId, ResourceSourceId};

/// ResourceSource - links a resource to its source page(s)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResourceSource {
    pub id: ResourceSourceId,
    pub resource_id: ResourceId,
    pub page_url: String,
    pub snapshot_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl ResourceSource {
    /// Find all sources for a resource
    pub async fn find_by_resource_id(resource_id: ResourceId, pool: &PgPool) -> Result<Vec<Self>> {
        let sources = sqlx::query_as::<_, Self>(
            "SELECT * FROM resource_sources WHERE resource_id = $1 ORDER BY created_at ASC",
        )
        .bind(resource_id)
        .fetch_all(pool)
        .await?;
        Ok(sources)
    }

    /// Find source URLs for a resource (convenience method)
    pub async fn find_urls_by_resource_id(resource_id: ResourceId, pool: &PgPool) -> Result<Vec<String>> {
        let urls = sqlx::query_scalar::<_, String>(
            "SELECT page_url FROM resource_sources WHERE resource_id = $1 ORDER BY created_at ASC",
        )
        .bind(resource_id)
        .fetch_all(pool)
        .await?;
        Ok(urls)
    }

    /// Check if a resource has a specific source URL
    pub async fn exists(resource_id: ResourceId, page_url: &str, pool: &PgPool) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM resource_sources WHERE resource_id = $1 AND page_url = $2)",
        )
        .bind(resource_id)
        .bind(page_url)
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }

    /// Add a source to a resource (upsert - ignores duplicates)
    pub async fn add(
        resource_id: ResourceId,
        page_url: String,
        snapshot_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        let source = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO resource_sources (resource_id, page_url, snapshot_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (resource_id, page_url) DO UPDATE
            SET snapshot_id = COALESCE(EXCLUDED.snapshot_id, resource_sources.snapshot_id)
            RETURNING *
            "#,
        )
        .bind(resource_id)
        .bind(page_url)
        .bind(snapshot_id)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Remove a source from a resource
    pub async fn remove(resource_id: ResourceId, page_url: &str, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM resource_sources WHERE resource_id = $1 AND page_url = $2")
            .bind(resource_id)
            .bind(page_url)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Remove all sources for a resource
    pub async fn remove_all(resource_id: ResourceId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM resource_sources WHERE resource_id = $1")
            .bind(resource_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
