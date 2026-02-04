//! ExtractionPage model
//!
//! Thin wrapper around the extraction library's `extraction_pages` table.
//! Used for querying pages by site/domain for post extraction.

use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// A page from the extraction library's storage.
///
/// This is a thin wrapper around `extraction_pages` table queries.
/// The extraction library manages this table; we only read from it.
pub struct ExtractionPage;

impl ExtractionPage {
    /// Get pages for a website domain for post extraction.
    ///
    /// Queries `extraction_pages` table for pages matching the domain's site_url.
    /// Returns tuples of (generated_id, url, content) for extraction.
    ///
    /// Note: The extraction library uses URL as primary key (TEXT).
    /// We generate deterministic UUIDs from URLs for compatibility with
    /// the agentic extraction pipeline.
    pub async fn find_by_domain(
        domain: &str,
        pool: &PgPool,
    ) -> Result<Vec<(Uuid, String, String)>> {
        // Normalize domain - remove protocol prefix if present
        let normalized = domain
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        // Build site_url patterns to match
        let https_prefix = format!("https://{}", normalized);
        let http_prefix = format!("http://{}", normalized);

        // Query extraction_pages table for pages belonging to this domain
        let pages = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT url, content
            FROM extraction_pages
            WHERE site_url = $1 OR site_url = $2
            ORDER BY fetched_at DESC
            "#,
        )
        .bind(&https_prefix)
        .bind(&http_prefix)
        .fetch_all(pool)
        .await?;

        // Convert to format expected by extract_from_website
        // Generate deterministic UUIDs from URLs for source tracking
        let result: Vec<(Uuid, String, String)> = pages
            .into_iter()
            .map(|(url, content)| {
                // Generate deterministic UUID from URL hash
                let id = Self::url_to_uuid(&url);
                (id, url, content)
            })
            .collect();

        Ok(result)
    }

    /// Count pages for a website domain.
    pub async fn count_by_domain(domain: &str, pool: &PgPool) -> Result<usize> {
        let normalized = domain
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        let https_prefix = format!("https://{}", normalized);
        let http_prefix = format!("http://{}", normalized);

        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM extraction_pages
            WHERE site_url = $1 OR site_url = $2
            "#,
        )
        .bind(&https_prefix)
        .bind(&http_prefix)
        .fetch_one(pool)
        .await?;

        Ok(count.0 as usize)
    }

    /// Generate a deterministic UUID from a URL.
    ///
    /// Uses SHA-256 hash of the URL, taking first 16 bytes as UUID.
    fn url_to_uuid(url: &str) -> Uuid {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = hasher.finalize();
        // Take first 16 bytes of hash as UUID
        let bytes: [u8; 16] = hash[..16].try_into().expect("hash is at least 16 bytes");
        Uuid::from_bytes(bytes)
    }
}
