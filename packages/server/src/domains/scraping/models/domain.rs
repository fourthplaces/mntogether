use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{DomainId, MemberId};

/// Domain - a website we scrape for listings (requires approval before crawling)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Domain {
    pub id: DomainId,
    pub domain_url: String,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub active: bool,

    // Approval workflow
    pub status: String, // 'pending_review', 'approved', 'rejected', 'suspended'
    pub submitted_by: Option<MemberId>,
    pub submitter_type: Option<String>, // 'admin', 'public_user', 'system'
    pub submission_context: Option<String>,
    pub reviewed_by: Option<MemberId>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    // Crawling configuration
    pub max_crawl_depth: i32,
    pub crawl_rate_limit_seconds: i32,
    pub is_trusted_domain: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Domain status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    PendingReview,
    Approved,
    Rejected,
    Suspended,
}

impl std::fmt::Display for DomainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainStatus::PendingReview => write!(f, "pending_review"),
            DomainStatus::Approved => write!(f, "approved"),
            DomainStatus::Rejected => write!(f, "rejected"),
            DomainStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for DomainStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_review" => Ok(DomainStatus::PendingReview),
            "approved" => Ok(DomainStatus::Approved),
            "rejected" => Ok(DomainStatus::Rejected),
            "suspended" => Ok(DomainStatus::Suspended),
            _ => Err(anyhow::anyhow!("Invalid domain status: {}", s)),
        }
    }
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
            "SELECT * FROM domains WHERE active = true AND status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;
        Ok(domains)
    }

    /// Find all approved domains (ready for crawling)
    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        let domains = sqlx::query_as::<_, Domain>(
            "SELECT * FROM domains WHERE status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;
        Ok(domains)
    }

    /// Find domains pending review
    pub async fn find_pending_review(pool: &PgPool) -> Result<Vec<Self>> {
        let domains = sqlx::query_as::<_, Domain>(
            "SELECT * FROM domains WHERE status = 'pending_review' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(domains)
    }

    /// Find approved domains due for scraping
    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let domains = sqlx::query_as::<_, Domain>(
            r#"
            SELECT * FROM domains
            WHERE status = 'approved'
              AND active = true
              AND (last_scraped_at IS NULL
                   OR last_scraped_at < NOW() - (scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_scraped_at NULLS FIRST
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(domains)
    }

    /// Find or create a domain (handles race conditions)
    ///
    /// This method uses INSERT ... ON CONFLICT to atomically handle concurrent
    /// requests. If the domain already exists, it returns the existing domain.
    /// This prevents duplicate key errors in high-concurrency scenarios.
    pub async fn find_or_create(
        domain_url: String,
        submitted_by: Option<MemberId>,
        submitter_type: String,
        submission_context: Option<String>,
        max_crawl_depth: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        // The create method now uses INSERT ... ON CONFLICT,
        // so it handles both creation and finding existing domains atomically
        Self::create(
            domain_url,
            submitted_by,
            submitter_type,
            submission_context,
            max_crawl_depth,
            pool,
        )
        .await
    }

    /// Create a new domain submission (starts as pending_review)
    ///
    /// Uses INSERT ... ON CONFLICT to handle concurrent requests gracefully.
    /// If the domain already exists, returns the existing domain.
    pub async fn create(
        domain_url: String,
        submitted_by: Option<MemberId>,
        submitter_type: String,
        submission_context: Option<String>,
        max_crawl_depth: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            INSERT INTO domains (
                domain_url,
                submitted_by,
                submitter_type,
                submission_context,
                max_crawl_depth,
                status
            )
            VALUES ($1, $2, $3, $4, $5, 'pending_review')
            ON CONFLICT (domain_url) DO UPDATE
            SET domain_url = EXCLUDED.domain_url  -- No-op update to return existing row
            RETURNING *
            "#,
        )
        .bind(domain_url)
        .bind(submitted_by)
        .bind(submitter_type)
        .bind(submission_context)
        .bind(max_crawl_depth)
        .fetch_one(pool)
        .await?;
        Ok(domain)
    }

    /// Approve a domain for crawling
    pub async fn approve(
        id: DomainId,
        reviewed_by: MemberId,
        pool: &PgPool,
    ) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            UPDATE domains
            SET
                status = 'approved',
                reviewed_by = $2,
                reviewed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .fetch_one(pool)
        .await?;
        Ok(domain)
    }

    /// Reject a domain submission
    pub async fn reject(
        id: DomainId,
        reviewed_by: MemberId,
        rejection_reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            UPDATE domains
            SET
                status = 'rejected',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(rejection_reason)
        .fetch_one(pool)
        .await?;
        Ok(domain)
    }

    /// Suspend an approved domain
    pub async fn suspend(
        id: DomainId,
        reviewed_by: MemberId,
        reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            UPDATE domains
            SET
                status = 'suspended',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(reason)
        .fetch_one(pool)
        .await?;
        Ok(domain)
    }

    /// Check if a domain is approved (for auto-approving URLs from this domain)
    pub async fn is_domain_approved(domain_url: &str, pool: &PgPool) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM domains WHERE domain_url = $1 AND status = 'approved')",
        )
        .bind(domain_url)
        .fetch_one(pool)
        .await?;
        Ok(result)
    }

    /// Mark domain as trusted (URLs from this domain are auto-approved)
    pub async fn mark_as_trusted(id: DomainId, pool: &PgPool) -> Result<Self> {
        let domain = sqlx::query_as::<_, Domain>(
            r#"
            UPDATE domains
            SET is_trusted_domain = true, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
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

    /// Extract domain from URL (e.g., "https://example.org/page" -> "example.org")
    pub fn extract_domain_from_url(url: &str) -> Result<String> {
        let url = url::Url::parse(url)?;
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in URL"))?
            .to_string();
        Ok(domain)
    }
}
