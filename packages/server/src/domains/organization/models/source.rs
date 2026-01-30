use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::SourceId;

/// Domain source - a website to scrape for resources (decoupled from organizations)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationSource {
    pub id: SourceId,
    pub domain_url: String,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl OrganizationSource {
    /// Find source by ID
    pub async fn find_by_id(id: SourceId, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM domains WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Find source by URL
    pub async fn find_by_url(url: &str, pool: &PgPool) -> Result<Option<Self>> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM domains WHERE domain_url = $1",
        )
        .bind(url)
        .fetch_optional(pool)
        .await?;
        Ok(source)
    }

    /// Find all active sources
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        let sources = sqlx::query_as::<_, OrganizationSource>(
            "SELECT * FROM domains WHERE active = true ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;
        Ok(sources)
    }

    /// Find sources due for scraping
    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let sources = sqlx::query_as::<_, OrganizationSource>(
            r#"
            SELECT * FROM domains
            WHERE active = true
              AND (last_scraped_at IS NULL
                   OR last_scraped_at < NOW() - (scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_scraped_at NULLS FIRST
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(sources)
    }

    /// Insert new source
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            r#"
            INSERT INTO domains (
                id, domain_url, scrape_frequency_hours, active, created_at
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(self.id)
        .bind(&self.domain_url)
        .bind(self.scrape_frequency_hours)
        .bind(self.active)
        .bind(self.created_at)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Update last_scraped_at timestamp
    pub async fn update_last_scraped(id: SourceId, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            r#"
            UPDATE domains
            SET last_scraped_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Set source active/inactive
    pub async fn set_active(id: SourceId, active: bool, pool: &PgPool) -> Result<Self> {
        let source = sqlx::query_as::<_, OrganizationSource>(
            r#"
            UPDATE domains
            SET active = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(active)
        .fetch_one(pool)
        .await?;
        Ok(source)
    }

    /// Add a URL to domain_scrape_urls table
    pub async fn add_scrape_url(id: SourceId, url: String, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO domain_scrape_urls (domain_id, url)
            VALUES ($1, $2)
            ON CONFLICT (domain_id, url) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(url)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Remove a URL from domain_scrape_urls table
    pub async fn remove_scrape_url(id: SourceId, url: String, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM domain_scrape_urls
            WHERE domain_id = $1 AND url = $2
            "#,
        )
        .bind(id)
        .bind(url)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get all scrape URLs for this domain from domain_scrape_urls table
    pub async fn get_scrape_urls(id: SourceId, pool: &PgPool) -> Result<Vec<String>> {
        let rows = sqlx::query!(
            r#"
            SELECT url FROM domain_scrape_urls
            WHERE domain_id = $1 AND active = true
            ORDER BY added_at
            "#,
            id.as_uuid()
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.url).collect())
    }
}
