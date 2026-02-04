//! PageSnapshot ingestor for reprocessing cached pages.
//!
//! This ingestor reads from the `page_snapshots` table and converts
//! entries to `RawPage` for processing through the extraction pipeline.
//!
//! Use this to migrate existing cached pages into the extraction library's
//! Summarize → Embed → Store pipeline without re-fetching from the web.

use async_trait::async_trait;
use extraction::error::{CrawlError, CrawlResult};
use extraction::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

use crate::common::WebsiteId;

/// Row type for the joined query
#[derive(Debug, sqlx::FromRow)]
struct PageSnapshotRow {
    pub id: Uuid,
    pub url: String,
    pub html: String,
    pub markdown: Option<String>,
    pub fetched_via: String,
    pub metadata: serde_json::Value,
    pub crawled_at: chrono::DateTime<chrono::Utc>,
}

/// Ingestor that reads from the `page_snapshots` table.
///
/// This bridges existing cached pages into the extraction pipeline,
/// allowing them to be summarized, embedded, and made searchable
/// without re-fetching from the web.
///
/// # Example
///
/// ```rust,ignore
/// use crate::domains::crawling::ingestors::PageSnapshotIngestor;
///
/// let ingestor = PageSnapshotIngestor::for_website(pool.clone(), website_id);
/// let config = DiscoverConfig::new("https://example.com").with_limit(100);
/// let pages = ingestor.discover(&config).await?;
/// // pages are now RawPage ready for extraction pipeline
/// ```
pub struct PageSnapshotIngestor {
    pool: PgPool,
    website_id: Option<WebsiteId>,
}

impl PageSnapshotIngestor {
    /// Create an ingestor for all page snapshots.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            website_id: None,
        }
    }

    /// Create an ingestor filtered to a specific website.
    ///
    /// Pages are found via the `website_snapshots` junction table.
    pub fn for_website(pool: PgPool, website_id: WebsiteId) -> Self {
        Self {
            pool,
            website_id: Some(website_id),
        }
    }

    /// Convert a PageSnapshotRow to RawPage.
    fn to_raw_page(row: PageSnapshotRow) -> RawPage {
        // Prefer markdown if available, fall back to HTML
        let content = row.markdown.unwrap_or_else(|| row.html.clone());

        // Extract title from HTML if present
        let title = extract_title(&row.html);

        // Convert JSON metadata to HashMap
        let mut metadata: HashMap<String, String> = row
            .metadata
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        metadata.insert("fetched_via".to_string(), row.fetched_via);
        metadata.insert("page_snapshot_id".to_string(), row.id.to_string());

        let mut page = RawPage::new(row.url, content)
            .with_fetched_at(row.crawled_at)
            .with_content_type("text/html".to_string());

        if let Some(title) = title {
            page = page.with_title(title);
        }

        page.metadata = metadata;
        page
    }
}

#[async_trait]
impl Ingestor for PageSnapshotIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        let limit = config.limit as i64;

        info!(
            website_id = ?self.website_id,
            limit = limit,
            "PageSnapshotIngestor.discover() starting"
        );

        let rows: Vec<PageSnapshotRow> = if let Some(website_id) = self.website_id {
            // Query via junction table for specific website
            let website_uuid = website_id.into_uuid();

            sqlx::query_as::<_, PageSnapshotRow>(
                r#"
                SELECT ps.id, ps.url, ps.html, ps.markdown, ps.fetched_via,
                       ps.metadata, ps.crawled_at
                FROM page_snapshots ps
                INNER JOIN website_snapshots ws ON ws.page_snapshot_id = ps.id
                WHERE ws.website_id = $1
                ORDER BY ps.crawled_at DESC
                LIMIT $2
                "#,
            )
            .bind(website_uuid)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?
        } else {
            // Query all page snapshots (filtered by URL pattern if provided)
            let url_pattern = format!("%{}%", config.url);

            sqlx::query_as::<_, PageSnapshotRow>(
                r#"
                SELECT id, url, html, markdown, fetched_via, metadata, crawled_at
                FROM page_snapshots
                WHERE url LIKE $1
                ORDER BY crawled_at DESC
                LIMIT $2
                "#,
            )
            .bind(&url_pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?
        };

        debug!(
            rows_found = rows.len(),
            "PageSnapshotIngestor fetched rows from database"
        );

        let pages: Vec<RawPage> = rows.into_iter().map(Self::to_raw_page).collect();

        info!(
            pages_count = pages.len(),
            "PageSnapshotIngestor.discover() completed"
        );

        Ok(pages)
    }

    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        if urls.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            url_count = urls.len(),
            "PageSnapshotIngestor.fetch_specific() starting"
        );

        // Query page_snapshots by URL
        // Note: Using ANY with array for efficient IN query
        let rows: Vec<PageSnapshotRow> = sqlx::query_as::<_, PageSnapshotRow>(
            r#"
            SELECT id, url, html, markdown, fetched_via, metadata, crawled_at
            FROM page_snapshots
            WHERE url = ANY($1)
            ORDER BY crawled_at DESC
            "#,
        )
        .bind(urls)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CrawlError::Http(Box::new(e)))?;

        let pages: Vec<RawPage> = rows.into_iter().map(Self::to_raw_page).collect();

        info!(
            pages_count = pages.len(),
            "PageSnapshotIngestor.fetch_specific() completed"
        );

        Ok(pages)
    }

    fn name(&self) -> &str {
        "page_snapshot"
    }
}

/// Extract title from HTML content.
fn extract_title(html: &str) -> Option<String> {
    let title_pattern = regex::Regex::new(r"<title[^>]*>(.*?)</title>").ok()?;
    title_pattern
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| {
            // Decode common HTML entities
            m.as_str()
                .trim()
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&nbsp;", " ")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        assert_eq!(
            extract_title("<html><head><title>My Page</title></head></html>"),
            Some("My Page".to_string())
        );

        assert_eq!(
            extract_title("<title>Test &amp; Demo</title>"),
            Some("Test & Demo".to_string())
        );

        assert_eq!(
            extract_title("<html><body>No title here</body></html>"),
            None
        );
    }
}
