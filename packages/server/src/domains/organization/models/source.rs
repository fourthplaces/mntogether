use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Organization source - a website to monitor for needs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationSource {
    pub id: Uuid,
    pub organization_name: String,
    pub source_url: String,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl OrganizationSource {
    /// Find source by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM organization_sources WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Find source by URL
    pub async fn find_by_url(url: &str, pool: &PgPool) -> Result<Option<Self>> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM organization_sources WHERE source_url = $1"
        )
        .bind(url)
        .fetch_optional(pool)
        .await?;
        Ok(source)
    }

    /// Find all active sources
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        let sources = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM organization_sources WHERE active = true ORDER BY created_at"
        )
        .fetch_all(pool)
        .await?;
        Ok(sources)
    }

    /// Find sources due for scraping
    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let sources = sqlx::query_as::<_, OrganizationSource>(
            r#"
            SELECT * FROM organization_sources
            WHERE active = true
              AND (last_scraped_at IS NULL
                   OR last_scraped_at < NOW() - (scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_scraped_at NULLS FIRST
            "#
        )
        .fetch_all(pool)
        .await?;
        Ok(sources)
    }

    /// Insert new source
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            r#"
            INSERT INTO organization_sources (
                id, organization_name, source_url, scrape_frequency_hours, active, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(self.id)
        .bind(&self.organization_name)
        .bind(&self.source_url)
        .bind(self.scrape_frequency_hours)
        .bind(self.active)
        .bind(self.created_at)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Update last_scraped_at timestamp
    pub async fn update_last_scraped(id: Uuid, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            r#"
            UPDATE organization_sources
            SET last_scraped_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Set source active/inactive
    pub async fn set_active(id: Uuid, active: bool, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            r#"
            UPDATE organization_sources
            SET active = $2
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(active)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }
}
