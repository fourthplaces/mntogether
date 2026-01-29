use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::DomainId;

/// Domain - a website we scrape for listings
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Domain {
    pub id: DomainId,
    pub domain_url: String,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DomainScrapeUrl - specific pages to scrape within a domain
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DomainScrapeUrl {
    pub id: uuid::Uuid,
    pub domain_id: DomainId,
    pub url: String,
    pub active: bool,
    pub added_at: DateTime<Utc>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Domain {
    /// Find domain by ID
    pub async fn find_by_id(id: DomainId, pool: &PgPool) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>("SELECT * FROM domains WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(domain)
    }

    /// Find domain by URL
    pub async fn find_by_url(url: &str, pool: &PgPool) -> Result<Option<Self>> {
        let domain = sqlx::query_as::<_, Domain>("SELECT * FROM domains WHERE domain_url = $1")
            .bind(url)
            .fetch_optional(pool)
            .await?;
        Ok(domain)
    }

    /// Find all active domains
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        let domains = sqlx::query_as::<_, Domain>(
            "SELECT * FROM domains WHERE active = true ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;
        Ok(domains)
    }

    /// Find domains due for scraping
    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let domains = sqlx::query_as::<_, Domain>(
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
        Ok(domains)
    }

    /// Create a new domain
    pub async fn create(
        domain_url: String,
        scrape_frequency_hours: i32,
        active: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            INSERT INTO domains (domain_url, scrape_frequency_hours, active)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(domain_url)
        .bind(scrape_frequency_hours)
        .bind(active)
        .fetch_one(pool)
        .await?;
        Ok(domain)
    }

    /// Update last_scraped_at timestamp
    pub async fn update_last_scraped(id: DomainId, pool: &PgPool) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
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
        Ok(domain)
    }

    /// Update scrape frequency
    pub async fn update_frequency(
        id: DomainId,
        scrape_frequency_hours: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            UPDATE domains
            SET scrape_frequency_hours = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(scrape_frequency_hours)
        .fetch_one(pool)
        .await?;
        Ok(domain)
    }

    /// Set domain active status
    pub async fn set_active(id: DomainId, active: bool, pool: &PgPool) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
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
        Ok(domain)
    }

    /// Delete a domain
    pub async fn delete(id: DomainId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM domains WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get scrape URLs for this domain
    pub async fn get_scrape_urls(&self, pool: &PgPool) -> Result<Vec<DomainScrapeUrl>> {
        DomainScrapeUrl::find_by_domain(self.id, pool).await
    }
}

impl DomainScrapeUrl {
    /// Find scrape URLs by domain ID
    pub async fn find_by_domain(domain_id: DomainId, pool: &PgPool) -> Result<Vec<Self>> {
        let urls = sqlx::query_as::<_, DomainScrapeUrl>(
            "SELECT * FROM domain_scrape_urls WHERE domain_id = $1 AND active = true",
        )
        .bind(domain_id)
        .fetch_all(pool)
        .await?;
        Ok(urls)
    }

    /// Add a scrape URL to a domain
    pub async fn create(domain_id: DomainId, url: String, pool: &PgPool) -> Result<Self> {
        let scrape_url = sqlx::query_as::<_, DomainScrapeUrl>(
            r#"
            INSERT INTO domain_scrape_urls (domain_id, url)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(domain_id)
        .bind(url)
        .fetch_one(pool)
        .await?;
        Ok(scrape_url)
    }

    /// Set scrape URL active status
    pub async fn set_active(id: uuid::Uuid, active: bool, pool: &PgPool) -> Result<Self> {
        let scrape_url = sqlx::query_as::<_, DomainScrapeUrl>(
            r#"
            UPDATE domain_scrape_urls
            SET active = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(active)
        .fetch_one(pool)
        .await?;
        Ok(scrape_url)
    }

    /// Delete a scrape URL
    pub async fn delete(id: uuid::Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM domain_scrape_urls WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
