use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{MemberId, WebsiteId};

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

    // Crawl tracking (for multi-page crawling with retry)
    pub crawl_status: Option<String>, // 'pending', 'crawling', 'completed', 'no_listings_found', 'failed'
    pub crawl_attempt_count: Option<i32>,
    pub max_crawl_retries: Option<i32>,
    pub last_crawl_started_at: Option<DateTime<Utc>>,
    pub last_crawl_completed_at: Option<DateTime<Utc>>,
    pub pages_crawled_count: Option<i32>,
    pub max_pages_per_crawl: Option<i32>,

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

/// Crawl status enum for multi-page crawling workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    Pending,
    Crawling,
    Completed,
    NoListingsFound,
    Failed,
}

impl std::fmt::Display for CrawlStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrawlStatus::Pending => write!(f, "pending"),
            CrawlStatus::Crawling => write!(f, "crawling"),
            CrawlStatus::Completed => write!(f, "completed"),
            CrawlStatus::NoListingsFound => write!(f, "no_listings_found"),
            CrawlStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for CrawlStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(CrawlStatus::Pending),
            "crawling" => Ok(CrawlStatus::Crawling),
            "completed" => Ok(CrawlStatus::Completed),
            "no_listings_found" => Ok(CrawlStatus::NoListingsFound),
            "failed" => Ok(CrawlStatus::Failed),
            _ => Err(anyhow::anyhow!("Invalid crawl status: {}", s)),
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
    pub async fn find_or_create(
        url: String,
        submitted_by: Option<MemberId>,
        submitter_type: String,
        submission_context: Option<String>,
        max_crawl_depth: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        // The create method now uses INSERT ... ON CONFLICT,
        // so it handles both creation and finding existing websites atomically
        Self::create(
            url,
            submitted_by,
            submitter_type,
            submission_context,
            max_crawl_depth,
            pool,
        )
        .await
    }

    /// Create a new website submission (starts as pending_review)
    ///
    /// Uses INSERT ... ON CONFLICT to handle concurrent requests gracefully.
    /// If the website already exists, returns the existing website.
    /// Input is normalized to just the domain (lowercase, no www prefix).
    pub async fn create(
        url_or_domain: String,
        submitted_by: Option<MemberId>,
        submitter_type: String,
        submission_context: Option<String>,
        max_crawl_depth: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        // Normalize to just the domain
        let normalized = Self::normalize_domain(&url_or_domain)?;

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
        .bind(submitted_by)
        .bind(submitter_type)
        .bind(submission_context)
        .bind(max_crawl_depth)
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
    // Crawl Tracking Methods
    // =========================================================================

    /// Mark website as starting a crawl
    pub async fn start_crawl(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET
                crawl_status = 'crawling',
                last_crawl_started_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Mark website crawl as complete with status and pages count
    pub async fn complete_crawl(
        id: WebsiteId,
        status: &str,
        pages_count: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET
                crawl_status = $2,
                pages_crawled_count = $3,
                last_crawl_completed_at = NOW(),
                last_scraped_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(pages_count)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Increment crawl attempt count and return new count
    pub async fn increment_crawl_attempt(id: WebsiteId, pool: &PgPool) -> Result<i32> {
        let result = sqlx::query_scalar::<_, i32>(
            r#"
            UPDATE websites
            SET
                crawl_attempt_count = COALESCE(crawl_attempt_count, 0) + 1,
                updated_at = NOW()
            WHERE id = $1
            RETURNING crawl_attempt_count
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(result)
    }

    /// Reset crawl attempt count (used after successful extraction)
    pub async fn reset_crawl_attempts(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET
                crawl_attempt_count = 0,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(website)
    }

    /// Check if website should retry crawl (has retries left)
    pub fn should_retry_crawl(&self) -> bool {
        let attempt_count = self.crawl_attempt_count.unwrap_or(0);
        let max_retries = self.max_crawl_retries.unwrap_or(5);
        attempt_count < max_retries
    }

    /// Find approved websites that need initial crawling
    pub async fn find_pending_initial_crawl(pool: &PgPool) -> Result<Vec<Self>> {
        let websites = sqlx::query_as::<_, Website>(
            r#"
            SELECT * FROM websites
            WHERE status = 'approved'
              AND active = true
              AND (crawl_status IS NULL OR crawl_status = 'pending')
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(websites)
    }

    /// Find approved websites due for re-crawling (completed crawls that are stale)
    pub async fn find_due_for_recrawl(pool: &PgPool) -> Result<Vec<Self>> {
        let websites = sqlx::query_as::<_, Website>(
            r#"
            SELECT * FROM websites
            WHERE status = 'approved'
              AND active = true
              AND crawl_status = 'completed'
              AND (last_crawl_completed_at IS NULL
                   OR last_crawl_completed_at < NOW() - (scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_crawl_completed_at NULLS FIRST
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(websites)
    }

    /// Update max pages per crawl setting
    pub async fn update_max_pages_per_crawl(
        id: WebsiteId,
        max_pages: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let website = sqlx::query_as::<_, Website>(
            r#"
            UPDATE websites
            SET
                max_pages_per_crawl = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(max_pages)
        .fetch_one(pool)
        .await?;
        Ok(website)
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
