use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::common::{MemberId, PaginationDirection, Readable, ValidatedPaginationArgs, WebsiteId};

/// Builder for creating a new Website
#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct CreateWebsite {
    pub url_or_domain: String,
    #[builder(default = "admin".to_string())]
    pub submitter_type: String,
    #[builder(default = 2)]
    pub max_crawl_depth: i32,
    #[builder(default)]
    pub submitted_by: Option<MemberId>,
    #[builder(default)]
    pub submission_context: Option<String>,
}

/// Website - a website we scrape for listings (requires approval before crawling)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Website {
    pub id: WebsiteId,
    pub domain: String,
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
    pub is_trusted: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Website status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebsiteStatus {
    PendingReview,
    Approved,
    Rejected,
    Suspended,
}

impl std::fmt::Display for WebsiteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebsiteStatus::PendingReview => write!(f, "pending_review"),
            WebsiteStatus::Approved => write!(f, "approved"),
            WebsiteStatus::Rejected => write!(f, "rejected"),
            WebsiteStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for WebsiteStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_review" => Ok(WebsiteStatus::PendingReview),
            "approved" => Ok(WebsiteStatus::Approved),
            "rejected" => Ok(WebsiteStatus::Rejected),
            "suspended" => Ok(WebsiteStatus::Suspended),
            _ => Err(anyhow::anyhow!("Invalid website status: {}", s)),
        }
    }
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Website {
    /// Find website by ID
    pub async fn find_by_id(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>("SELECT * FROM websites WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(website)
    }

    /// Find website by domain (normalizes the input URL/domain before searching)
    pub async fn find_by_domain(url_or_domain: &str, pool: &PgPool) -> Result<Option<Self>> {
        let normalized = Self::normalize_domain(url_or_domain)?;
        let website = sqlx::query_as::<_, Website>("SELECT * FROM websites WHERE domain = $1")
            .bind(normalized)
            .fetch_optional(pool)
            .await?;
        Ok(website)
    }

    /// Find all active websites
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        let websites = sqlx::query_as::<_, Website>(
            "SELECT * FROM websites WHERE active = true AND status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;
        Ok(websites)
    }

    /// Find all approved websites (ready for crawling)
    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        let websites = sqlx::query_as::<_, Website>(
            "SELECT * FROM websites WHERE status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await?;
        Ok(websites)
    }

    /// Find websites pending review
    pub async fn find_pending_review(pool: &PgPool) -> Result<Vec<Self>> {
        let websites = sqlx::query_as::<_, Website>(
            "SELECT * FROM websites WHERE status = 'pending_review' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(websites)
    }

    /// Find approved websites due for scraping
    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let websites = sqlx::query_as::<_, Website>(
            r#"
            SELECT * FROM websites
            WHERE status = 'approved'
              AND active = true
              AND (last_scraped_at IS NULL
                   OR last_scraped_at < NOW() - (scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_scraped_at NULLS FIRST
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(websites)
    }

    /// Find or create a website (handles race conditions)
    ///
    /// This method uses INSERT ... ON CONFLICT to atomically handle concurrent
    /// requests. If the website already exists, it returns the existing website.
    /// This prevents duplicate key errors in high-concurrency scenarios.
    pub async fn find_or_create(input: CreateWebsite, pool: &PgPool) -> Result<Self> {
        // The create method now uses INSERT ... ON CONFLICT,
        // so it handles both creation and finding existing websites atomically
        Self::create(input, pool).await
    }

    /// Create a new website submission (starts as pending_review)
    ///
    /// Uses INSERT ... ON CONFLICT to handle concurrent requests gracefully.
    /// If the website already exists, returns the existing website.
    /// Input is normalized to just the domain (lowercase, no www prefix).
    pub async fn create(input: CreateWebsite, pool: &PgPool) -> Result<Self> {
        // Normalize to just the domain
        let normalized = Self::normalize_domain(&input.url_or_domain)?;

        let website = sqlx::query_as::<_, Website>(
            r#"
            INSERT INTO websites (
                domain,
                submitted_by,
                submitter_type,
                submission_context,
                max_crawl_depth,
                status
            )
            VALUES ($1, $2, $3, $4, $5, 'pending_review')
            ON CONFLICT (domain) DO UPDATE
            SET domain = EXCLUDED.domain  -- No-op update to return existing row
            RETURNING *
            "#,
        )
        .bind(normalized)
        .bind(input.submitted_by)
        .bind(input.submitter_type)
        .bind(input.submission_context)
        .bind(input.max_crawl_depth)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Approve a website for crawling
    pub async fn approve(id: WebsiteId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
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
        Ok(website)
    }

    /// Reject a website submission
    pub async fn reject(
        id: WebsiteId,
        reviewed_by: MemberId,
        rejection_reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
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
        Ok(website)
    }

    /// Suspend an approved website
    pub async fn suspend(
        id: WebsiteId,
        reviewed_by: MemberId,
        reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
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
        Ok(website)
    }

    /// Check if a website is approved (for auto-approving URLs from this domain)
    pub async fn is_domain_approved(url_or_domain: &str, pool: &PgPool) -> Result<bool> {
        let normalized = Self::normalize_domain(url_or_domain)?;
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM websites WHERE domain = $1 AND status = 'approved')",
        )
        .bind(normalized)
        .fetch_one(pool)
        .await?;
        Ok(result)
    }

    /// Mark website as trusted (URLs from this website are auto-approved)
    pub async fn mark_as_trusted(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET is_trusted = true, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Update last_scraped_at timestamp
    pub async fn update_last_scraped(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET last_scraped_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Update scrape frequency
    pub async fn update_frequency(
        id: WebsiteId,
        scrape_frequency_hours: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET scrape_frequency_hours = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(scrape_frequency_hours)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Set website active status
    pub async fn set_active(id: WebsiteId, active: bool, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET active = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(active)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Delete a website
    pub async fn delete(id: WebsiteId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM websites WHERE id = $1")
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

    /// Normalize a URL or domain to just the domain for consistent storage
    ///
    /// Examples:
    /// - "https://www.example.org/page" -> "example.org"
    /// - "http://EXAMPLE.ORG" -> "example.org"
    /// - "www.example.org" -> "example.org"
    /// - "example.org" -> "example.org"
    pub fn normalize_domain(url_or_domain: &str) -> Result<String> {
        let input = url_or_domain.trim();

        // If no protocol, try adding https:// to parse it
        let with_protocol = if input.starts_with("http://") || input.starts_with("https://") {
            input.to_string()
        } else {
            format!("https://{}", input)
        };

        let parsed = url::Url::parse(&with_protocol)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in input: {}", url_or_domain))?;

        // Normalize: lowercase and strip www. prefix
        let normalized = host
            .to_lowercase()
            .strip_prefix("www.")
            .map(|s| s.to_string())
            .unwrap_or_else(|| host.to_lowercase());

        Ok(normalized)
    }

    // =========================================================================
    // Cursor-Based Pagination (Relay spec)
    // =========================================================================

    /// Find websites with cursor-based pagination
    pub async fn find_paginated(
        status: Option<&str>,
        search: Option<&str>,
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    r#"
                    SELECT * FROM websites
                    WHERE ($1::text IS NULL OR status = $1)
                      AND ($2::uuid IS NULL OR id > $2)
                      AND ($4::text IS NULL OR domain ILIKE '%' || $4 || '%')
                    ORDER BY id ASC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(search)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                let mut rows = sqlx::query_as::<_, Self>(
                    r#"
                    SELECT * FROM websites
                    WHERE ($1::text IS NULL OR status = $1)
                      AND ($2::uuid IS NULL OR id < $2)
                      AND ($4::text IS NULL OR domain ILIKE '%' || $4 || '%')
                    ORDER BY id DESC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(search)
                .fetch_all(pool)
                .await?;

                rows.reverse();
                rows
            }
        };

        let has_more = results.len() > args.limit as usize;
        let results = if has_more {
            results.into_iter().take(args.limit as usize).collect()
        } else {
            results
        };

        Ok((results, has_more))
    }

    /// Batch-lookup website domains by IDs
    pub async fn find_domains_by_ids(ids: &[Uuid], pool: &PgPool) -> Result<Vec<(Uuid, String)>> {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, domain FROM websites WHERE id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Count websites with optional status and search filters
    pub async fn count_with_filters(
        status: Option<&str>,
        search: Option<&str>,
        pool: &PgPool,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM websites WHERE ($1::text IS NULL OR status = $1) AND ($2::text IS NULL OR domain ILIKE '%' || $2 || '%')",
        )
        .bind(status)
        .bind(search)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}

// Implement Readable trait for ReadResult pattern
#[async_trait]
impl Readable for Website {
    type Id = WebsiteId;

    async fn read_by_id(id: Self::Id, pool: &PgPool) -> Result<Option<Self>> {
        Self::find_by_id(id, pool).await.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_domain() {
        // Full URLs with protocol
        assert_eq!(
            Website::normalize_domain("https://www.example.org/page").unwrap(),
            "example.org"
        );
        assert_eq!(
            Website::normalize_domain("http://www.example.org").unwrap(),
            "example.org"
        );
        assert_eq!(
            Website::normalize_domain("https://example.org/path/to/page").unwrap(),
            "example.org"
        );

        // Without protocol
        assert_eq!(
            Website::normalize_domain("www.example.org").unwrap(),
            "example.org"
        );
        assert_eq!(
            Website::normalize_domain("example.org").unwrap(),
            "example.org"
        );

        // Uppercase should be lowercased
        assert_eq!(
            Website::normalize_domain("https://WWW.EXAMPLE.ORG").unwrap(),
            "example.org"
        );
        assert_eq!(
            Website::normalize_domain("EXAMPLE.ORG").unwrap(),
            "example.org"
        );

        // Subdomains (not www) should be preserved
        assert_eq!(
            Website::normalize_domain("https://blog.example.org").unwrap(),
            "blog.example.org"
        );
        assert_eq!(
            Website::normalize_domain("https://www.blog.example.org").unwrap(),
            "blog.example.org"
        );

        // Real-world examples
        assert_eq!(
            Website::normalize_domain("https://www.dhhmn.com/").unwrap(),
            "dhhmn.com"
        );
        assert_eq!(
            Website::normalize_domain("http://dhhmn.com").unwrap(),
            "dhhmn.com"
        );

        // Whitespace should be trimmed
        assert_eq!(
            Website::normalize_domain("  https://example.org  ").unwrap(),
            "example.org"
        );
    }
}
