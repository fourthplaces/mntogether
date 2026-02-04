//! Ingestor trait for pluggable data ingestion.
//!
//! The Ingestor pattern provides a unified interface for fetching content
//! from various sources (web, PDFs, S3, databases, etc.) into the extraction
//! pipeline.
//!
//! # Philosophy
//!
//! "Ingestor" is preferred over "Crawler" because:
//! - Crawling implies web-only; ingestion is source-agnostic
//! - Supports PDFs, documents, S3 buckets, databases, APIs
//! - Better aligns with the "Reasoning Search Engine" architecture
//!
//! # Usage
//!
//! ```rust,ignore
//! use extraction::traits::ingestor::{Ingestor, RawPage};
//!
//! // Discover pages from a site
//! let pages = ingestor.discover("https://example.com", 10).await?;
//!
//! // Fetch specific URLs (for Detective gap-filling)
//! let specific = ingestor.fetch_specific(&["https://example.com/about"]).await?;
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{CrawlError, CrawlResult};
use crate::traits::crawler::UrlValidator;

/// Raw page content before Index processing.
///
/// This is the output from Ingestors - raw content that hasn't been
/// summarized or embedded yet. It will be processed through the
/// Summarize → Embed → Store pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPage {
    /// URL or identifier for this content
    pub url: String,

    /// Raw content (markdown, HTML, or plain text)
    pub content: String,

    /// Content title if available
    pub title: Option<String>,

    /// MIME type or content type (e.g., "text/html", "application/pdf")
    pub content_type: Option<String>,

    /// When the content was fetched
    pub fetched_at: DateTime<Utc>,

    /// Source-specific metadata (e.g., HTTP headers, S3 metadata)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl RawPage {
    /// Create a new raw page with minimal fields.
    pub fn new(url: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            content: content.into(),
            title: None,
            content_type: None,
            fetched_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Set the page title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the content type.
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Set the fetched timestamp.
    pub fn with_fetched_at(mut self, fetched_at: DateTime<Utc>) -> Self {
        self.fetched_at = fetched_at;
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get content length in bytes.
    pub fn content_length(&self) -> usize {
        self.content.len()
    }

    /// Check if this page has content.
    pub fn has_content(&self) -> bool {
        !self.content.trim().is_empty()
    }

    /// Extract the site URL from this page's URL.
    pub fn site_url(&self) -> Option<String> {
        url::Url::parse(&self.url)
            .ok()
            .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or("")))
    }
}

/// Configuration for discovery operations.
#[derive(Debug, Clone)]
pub struct DiscoverConfig {
    /// Starting URL or identifier
    pub url: String,

    /// Maximum number of pages to discover
    pub limit: usize,

    /// Maximum depth for recursive discovery (0 = single page)
    pub max_depth: usize,

    /// URL patterns to include (glob patterns)
    pub include_patterns: Vec<String>,

    /// URL patterns to exclude (glob patterns)
    pub exclude_patterns: Vec<String>,

    /// Additional options (source-specific)
    pub options: HashMap<String, String>,
}

impl DiscoverConfig {
    /// Create a new config for discovering from a URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            limit: 100,
            max_depth: 2,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            options: HashMap::new(),
        }
    }

    /// Set the page limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set the max depth.
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Add an include pattern.
    pub fn include(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// Add an exclude pattern.
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// Add a source-specific option.
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Ingestor trait for pluggable content ingestion.
///
/// Implementations fetch content from various sources:
/// - `HttpIngestor` - Basic HTTP crawling
/// - `FirecrawlIngestor` - Firecrawl API (JavaScript rendering, anti-bot)
/// - `S3Ingestor` - AWS S3 buckets
/// - `PdfIngestor` - PDF documents
///
/// # SSRF Protection
///
/// Always wrap URL-based ingestors with `ValidatedIngestor` in production:
///
/// ```rust,ignore
/// let ingestor = ValidatedIngestor::new(HttpIngestor::new());
/// ```
#[async_trait]
pub trait Ingestor: Send + Sync {
    /// Discover and fetch pages from a source.
    ///
    /// This is the primary method for initial ingestion. It:
    /// 1. Starts at the given URL
    /// 2. Discovers related pages (crawling, sitemap, etc.)
    /// 3. Returns up to `config.limit` raw pages
    ///
    /// # Arguments
    ///
    /// * `config` - Discovery configuration (URL, limits, patterns)
    ///
    /// # Returns
    ///
    /// Vector of raw pages ready for processing.
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>>;

    /// Fetch specific URLs.
    ///
    /// Used for:
    /// - Detective gap-filling (fetch specific pages to answer questions)
    /// - Re-fetching stale pages
    /// - User-submitted URLs
    ///
    /// # Arguments
    ///
    /// * `urls` - Specific URLs to fetch
    ///
    /// # Returns
    ///
    /// Vector of raw pages (may be fewer than requested if some fail).
    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>>;

    /// Fetch a single URL.
    ///
    /// Convenience method that calls `fetch_specific` with one URL.
    async fn fetch_one(&self, url: &str) -> CrawlResult<RawPage> {
        let pages = self.fetch_specific(&[url.to_string()]).await?;
        pages
            .into_iter()
            .next()
            .ok_or_else(|| CrawlError::Http(format!("Failed to fetch {}", url).into()))
    }

    /// Get the ingestor name (for logging/debugging).
    fn name(&self) -> &str {
        "unknown"
    }
}

/// An ingestor that validates URLs before fetching (SSRF protection).
///
/// Wraps any URL-based ingestor to ensure all URLs are validated
/// before fetching. This prevents Server-Side Request Forgery attacks.
///
/// # Example
///
/// ```rust,ignore
/// let ingestor = ValidatedIngestor::new(HttpIngestor::new());
/// // All URLs will be validated before fetching
/// let pages = ingestor.discover(&config).await?;
/// ```
pub struct ValidatedIngestor<I: Ingestor> {
    inner: I,
    validator: UrlValidator,
}

impl<I: Ingestor> ValidatedIngestor<I> {
    /// Create a new validated ingestor with default security rules.
    pub fn new(ingestor: I) -> Self {
        Self {
            inner: ingestor,
            validator: UrlValidator::new(),
        }
    }

    /// Create with a custom validator.
    pub fn with_validator(ingestor: I, validator: UrlValidator) -> Self {
        Self {
            inner: ingestor,
            validator,
        }
    }

    /// Validate a URL, returning an error if blocked.
    async fn validate_url(&self, url: &str) -> CrawlResult<()> {
        self.validator
            .validate_with_dns(url)
            .await
            .map_err(CrawlError::Security)
    }
}

#[async_trait]
impl<I: Ingestor> Ingestor for ValidatedIngestor<I> {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        // Validate the starting URL
        self.validate_url(&config.url).await?;

        // Discover pages
        let pages = self.inner.discover(config).await?;

        // Filter out any pages with invalid URLs (in case of redirects)
        let validated: Vec<_> = pages
            .into_iter()
            .filter(|p| self.validator.validate(&p.url).is_ok())
            .collect();

        Ok(validated)
    }

    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        // Validate all URLs first
        let mut valid_urls = Vec::with_capacity(urls.len());
        for url in urls {
            if let Err(e) = self.validate_url(url).await {
                tracing::warn!("Skipping blocked URL {}: {}", url, e);
                continue;
            }
            valid_urls.push(url.clone());
        }

        if valid_urls.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch validated URLs
        let pages = self.inner.fetch_specific(&valid_urls).await?;

        // Double-check results (in case of redirects)
        let validated: Vec<_> = pages
            .into_iter()
            .filter(|p| self.validator.validate(&p.url).is_ok())
            .collect();

        Ok(validated)
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_page_builder() {
        let page = RawPage::new("https://example.com", "Hello, world!")
            .with_title("Example")
            .with_content_type("text/html")
            .with_metadata("source", "test");

        assert_eq!(page.url, "https://example.com");
        assert_eq!(page.title, Some("Example".to_string()));
        assert_eq!(page.content_type, Some("text/html".to_string()));
        assert_eq!(page.metadata.get("source"), Some(&"test".to_string()));
        assert!(page.has_content());
    }

    #[test]
    fn test_discover_config_builder() {
        let config = DiscoverConfig::new("https://example.com")
            .with_limit(50)
            .with_max_depth(3)
            .include("*/blog/*")
            .exclude("*/admin/*")
            .with_option("scrape_formats", "markdown");

        assert_eq!(config.url, "https://example.com");
        assert_eq!(config.limit, 50);
        assert_eq!(config.max_depth, 3);
        assert!(config.include_patterns.contains(&"*/blog/*".to_string()));
        assert!(config.exclude_patterns.contains(&"*/admin/*".to_string()));
        assert_eq!(
            config.options.get("scrape_formats"),
            Some(&"markdown".to_string())
        );
    }

    #[test]
    fn test_site_url_extraction() {
        let page = RawPage::new("https://example.com/path/to/page", "content");
        assert_eq!(page.site_url(), Some("https://example.com".to_string()));
    }

    #[test]
    fn test_empty_content_detection() {
        let empty = RawPage::new("https://example.com", "   ");
        assert!(!empty.has_content());

        let has_content = RawPage::new("https://example.com", "Hello");
        assert!(has_content.has_content());
    }
}
