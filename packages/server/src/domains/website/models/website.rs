use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::common::{
    MemberId, OrganizationId, PaginationDirection, Readable, ValidatedPaginationArgs, WebsiteId,
};

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

    // Organization linkage
    pub organization_id: Option<OrganizationId>,

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
// Queries target: sources s JOIN website_sources ws ON ws.source_id = s.id
// =============================================================================

impl Website {
    /// Base SELECT query for Website from joined sources + website_sources.
    /// Pass a WHERE/ORDER/LIMIT suffix to complete the query.
    fn base_query(suffix: &str) -> String {
        format!(
            "SELECT s.id, ws.domain, s.scrape_frequency_hours, s.last_scraped_at, s.active, \
             s.status, s.submitted_by, s.submitter_type, s.submission_context, \
             s.reviewed_by, s.reviewed_at, s.rejection_reason, \
             ws.max_crawl_depth, ws.crawl_rate_limit_seconds, ws.is_trusted, \
             s.organization_id, s.created_at, s.updated_at \
             FROM sources s JOIN website_sources ws ON ws.source_id = s.id {}",
            suffix
        )
    }

    /// Find website by ID
    pub async fn find_by_id(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let q = Self::base_query("WHERE s.id = $1");
        let website = sqlx::query_as::<_, Website>(&q)
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(website)
    }

    /// Find website by domain (normalizes the input URL/domain before searching)
    pub async fn find_by_domain(url_or_domain: &str, pool: &PgPool) -> Result<Option<Self>> {
        let normalized = Self::normalize_domain(url_or_domain)?;
        let q = Self::base_query("WHERE ws.domain = $1");
        let website = sqlx::query_as::<_, Website>(&q)
            .bind(normalized)
            .fetch_optional(pool)
            .await?;
        Ok(website)
    }

    /// Find all active websites
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query(
            "WHERE s.active = true AND s.status = 'approved' ORDER BY s.created_at",
        );
        let websites = sqlx::query_as::<_, Website>(&q).fetch_all(pool).await?;
        Ok(websites)
    }

    /// Find all approved websites (ready for crawling)
    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query("WHERE s.status = 'approved' ORDER BY s.created_at");
        let websites = sqlx::query_as::<_, Website>(&q).fetch_all(pool).await?;
        Ok(websites)
    }

    /// Find websites pending review
    pub async fn find_pending_review(pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query("WHERE s.status = 'pending_review' ORDER BY s.created_at DESC");
        let websites = sqlx::query_as::<_, Website>(&q).fetch_all(pool).await?;
        Ok(websites)
    }

    /// Find approved websites due for scraping
    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query(
            r#"WHERE s.status = 'approved'
              AND s.active = true
              AND (s.last_scraped_at IS NULL
                   OR s.last_scraped_at < NOW() - (s.scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY s.last_scraped_at NULLS FIRST"#,
        );
        let websites = sqlx::query_as::<_, Website>(&q).fetch_all(pool).await?;
        Ok(websites)
    }

    /// Find or create a website (handles race conditions)
    pub async fn find_or_create(input: CreateWebsite, pool: &PgPool) -> Result<Self> {
        Self::create(input, pool).await
    }

    /// Create a new website submission (starts as pending_review)
    ///
    /// If the website already exists (by domain), returns the existing website.
    /// Input is normalized to just the domain (lowercase, no www prefix).
    pub async fn create(input: CreateWebsite, pool: &PgPool) -> Result<Self> {
        let normalized = Self::normalize_domain(&input.url_or_domain)?;

        // Return existing if found
        let q = Self::base_query("WHERE ws.domain = $1");
        if let Some(existing) = sqlx::query_as::<_, Self>(&q)
            .bind(&normalized)
            .fetch_optional(pool)
            .await?
        {
            return Ok(existing);
        }

        // Create new website in a transaction (sources + website_sources)
        let mut tx = pool.begin().await?;

        let source_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO sources (source_type, submitted_by, submitter_type, submission_context, status)
             VALUES ('website', $1, $2, $3, 'pending_review')
             RETURNING id",
        )
        .bind(input.submitted_by)
        .bind(&input.submitter_type)
        .bind(&input.submission_context)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO website_sources (source_id, domain, max_crawl_depth)
             VALUES ($1, $2, $3)",
        )
        .bind(source_id)
        .bind(&normalized)
        .bind(input.max_crawl_depth)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Self::find_by_id(WebsiteId::from_uuid(source_id), pool).await
    }

    /// Approve a website for crawling
    pub async fn approve(id: WebsiteId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        sqlx::query(
            "UPDATE sources SET status = 'approved', reviewed_by = $2, reviewed_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(reviewed_by)
        .execute(pool)
        .await?;
        Self::find_by_id(id, pool).await
    }

    /// Reject a website submission
    pub async fn reject(
        id: WebsiteId,
        reviewed_by: MemberId,
        rejection_reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query(
            "UPDATE sources SET status = 'rejected', reviewed_by = $2, reviewed_at = NOW(), rejection_reason = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(rejection_reason)
        .execute(pool)
        .await?;
        Self::find_by_id(id, pool).await
    }

    /// Suspend an approved website
    pub async fn suspend(
        id: WebsiteId,
        reviewed_by: MemberId,
        reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query(
            "UPDATE sources SET status = 'suspended', reviewed_by = $2, reviewed_at = NOW(), rejection_reason = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(reason)
        .execute(pool)
        .await?;
        Self::find_by_id(id, pool).await
    }

    /// Check if a website is approved (for auto-approving URLs from this domain)
    pub async fn is_domain_approved(url_or_domain: &str, pool: &PgPool) -> Result<bool> {
        let normalized = Self::normalize_domain(url_or_domain)?;
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM website_sources ws JOIN sources s ON s.id = ws.source_id WHERE ws.domain = $1 AND s.status = 'approved')",
        )
        .bind(normalized)
        .fetch_one(pool)
        .await?;
        Ok(result)
    }

    /// Mark website as trusted (URLs from this website are auto-approved)
    pub async fn mark_as_trusted(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        sqlx::query("UPDATE website_sources SET is_trusted = true WHERE source_id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("UPDATE sources SET updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Self::find_by_id(id, pool).await
    }

    /// Update last_scraped_at timestamp
    pub async fn update_last_scraped(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        sqlx::query("UPDATE sources SET last_scraped_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Self::find_by_id(id, pool).await
    }

    /// Update scrape frequency
    pub async fn update_frequency(
        id: WebsiteId,
        scrape_frequency_hours: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query(
            "UPDATE sources SET scrape_frequency_hours = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(scrape_frequency_hours)
        .execute(pool)
        .await?;
        Self::find_by_id(id, pool).await
    }

    /// Set website active status
    pub async fn set_active(id: WebsiteId, active: bool, pool: &PgPool) -> Result<Self> {
        sqlx::query("UPDATE sources SET active = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(active)
            .execute(pool)
            .await?;
        Self::find_by_id(id, pool).await
    }

    /// Set the organization for a website
    pub async fn set_organization_id(
        id: WebsiteId,
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query("UPDATE sources SET organization_id = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(organization_id)
            .execute(pool)
            .await?;
        Self::find_by_id(id, pool).await
    }

    /// Find all websites belonging to an organization
    pub async fn find_by_organization(org_id: OrganizationId, pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query("WHERE s.organization_id = $1 ORDER BY ws.domain");
        sqlx::query_as::<_, Self>(&q)
            .bind(org_id)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find websites without an organization
    pub async fn find_without_organization(pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query(
            "WHERE s.organization_id IS NULL AND s.status = 'approved' ORDER BY ws.domain",
        );
        sqlx::query_as::<_, Self>(&q)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn unset_organization_id(id: WebsiteId, pool: &PgPool) -> Result<Self> {
        sqlx::query("UPDATE sources SET organization_id = NULL, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Self::find_by_id(id, pool).await
    }

    /// Delete a website
    pub async fn delete(id: WebsiteId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM sources WHERE id = $1")
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
        organization_id: Option<Uuid>,
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                let q = Self::base_query(
                    r#"WHERE ($1::text IS NULL OR s.status = $1)
                      AND ($2::uuid IS NULL OR s.id > $2)
                      AND ($4::text IS NULL OR ws.domain ILIKE '%' || $4 || '%')
                      AND ($5::uuid IS NULL OR s.organization_id = $5)
                    ORDER BY s.id ASC
                    LIMIT $3"#,
                );
                sqlx::query_as::<_, Self>(&q)
                    .bind(status)
                    .bind(args.cursor)
                    .bind(fetch_limit)
                    .bind(search)
                    .bind(organization_id)
                    .fetch_all(pool)
                    .await?
            }
            PaginationDirection::Backward => {
                let q = Self::base_query(
                    r#"WHERE ($1::text IS NULL OR s.status = $1)
                      AND ($2::uuid IS NULL OR s.id < $2)
                      AND ($4::text IS NULL OR ws.domain ILIKE '%' || $4 || '%')
                      AND ($5::uuid IS NULL OR s.organization_id = $5)
                    ORDER BY s.id DESC
                    LIMIT $3"#,
                );
                let mut rows = sqlx::query_as::<_, Self>(&q)
                    .bind(status)
                    .bind(args.cursor)
                    .bind(fetch_limit)
                    .bind(search)
                    .bind(organization_id)
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
            "SELECT source_id, domain FROM website_sources WHERE source_id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Count websites with optional status, search, and organization filters
    pub async fn count_with_filters(
        status: Option<&str>,
        search: Option<&str>,
        organization_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*)
            FROM sources s JOIN website_sources ws ON ws.source_id = s.id
            WHERE ($1::text IS NULL OR s.status = $1)
              AND ($2::text IS NULL OR ws.domain ILIKE '%' || $2 || '%')
              AND ($3::uuid IS NULL OR s.organization_id = $3)"#,
        )
        .bind(status)
        .bind(search)
        .bind(organization_id)
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
