use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{SourceId, WebsiteSourceId};

/// Website-specific source details (1:1 extension of sources)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebsiteSource {
    pub id: WebsiteSourceId,
    pub source_id: SourceId,
    pub domain: String,
    pub max_crawl_depth: i32,
    pub crawl_rate_limit_seconds: i32,
    pub is_trusted: bool,
}

impl WebsiteSource {
    pub async fn find_by_source_id(source_id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM website_sources WHERE source_id = $1")
            .bind(source_id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_source_id_optional(
        source_id: SourceId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM website_sources WHERE source_id = $1")
            .bind(source_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_domain(domain: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM website_sources WHERE domain = $1")
            .bind(domain)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn create(
        source_id: SourceId,
        domain: &str,
        max_crawl_depth: i32,
        crawl_rate_limit_seconds: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO website_sources (source_id, domain, max_crawl_depth, crawl_rate_limit_seconds)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(source_id)
        .bind(domain)
        .bind(max_crawl_depth)
        .bind(crawl_rate_limit_seconds)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn mark_as_trusted(source_id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE website_sources SET is_trusted = true WHERE source_id = $1 RETURNING *",
        )
        .bind(source_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Check if a domain belongs to an approved source
    pub async fn is_domain_approved(domain: &str, pool: &PgPool) -> Result<bool> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM website_sources ws
                JOIN sources s ON s.id = ws.source_id
                WHERE ws.domain = $1 AND s.status = 'approved'
            )
            "#,
        )
        .bind(domain)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Normalize a URL or domain to just the domain for consistent storage
    pub fn normalize_domain(url_or_domain: &str) -> Result<String> {
        let input = url_or_domain.trim();

        let with_protocol = if input.starts_with("http://") || input.starts_with("https://") {
            input.to_string()
        } else {
            format!("https://{}", input)
        };

        let parsed = url::Url::parse(&with_protocol)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in input: {}", url_or_domain))?;

        let normalized = host
            .to_lowercase()
            .strip_prefix("www.")
            .map(|s| s.to_string())
            .unwrap_or_else(|| host.to_lowercase());

        Ok(normalized)
    }

    /// Batch-lookup domains by source IDs
    pub async fn find_domains_by_source_ids(
        ids: &[uuid::Uuid],
        pool: &PgPool,
    ) -> Result<Vec<(uuid::Uuid, String)>> {
        sqlx::query_as::<_, (uuid::Uuid, String)>(
            "SELECT source_id, domain FROM website_sources WHERE source_id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_domain() {
        assert_eq!(
            WebsiteSource::normalize_domain("https://www.example.org/page").unwrap(),
            "example.org"
        );
        assert_eq!(
            WebsiteSource::normalize_domain("example.org").unwrap(),
            "example.org"
        );
        assert_eq!(
            WebsiteSource::normalize_domain("https://WWW.EXAMPLE.ORG").unwrap(),
            "example.org"
        );
        assert_eq!(
            WebsiteSource::normalize_domain("https://blog.example.org").unwrap(),
            "blog.example.org"
        );
    }
}
