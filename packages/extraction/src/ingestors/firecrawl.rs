//! Firecrawl-based ingestor implementation.
//!
//! Uses the Firecrawl API for crawling JavaScript-heavy sites with
//! anti-bot protection and JavaScript rendering.
//!
//! Requires the `firecrawl` feature to be enabled.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{CrawlError, CrawlResult};
use crate::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};

const FIRECRAWL_API_URL: &str = "https://api.firecrawl.dev/v1";

/// Firecrawl-based ingestor for JavaScript-heavy sites.
///
/// Uses the Firecrawl API which provides:
/// - JavaScript rendering
/// - Anti-bot protection bypass
/// - Automatic content extraction
/// - Markdown conversion
///
/// # Example
///
/// ```rust,ignore
/// use extraction::ingestors::{FirecrawlIngestor, ValidatedIngestor, DiscoverConfig};
///
/// let api_key = std::env::var("FIRECRAWL_API_KEY").unwrap();
/// let ingestor = ValidatedIngestor::new(FirecrawlIngestor::new(api_key)?);
/// let config = DiscoverConfig::new("https://example.com").with_limit(10);
/// let pages = ingestor.discover(&config).await?;
/// ```
pub struct FirecrawlIngestor {
    client: Client,
    api_key: String,
    /// Timeout for polling crawl status (seconds)
    poll_timeout_secs: u64,
    /// Interval between poll attempts (seconds)
    poll_interval_secs: u64,
}

// Request/Response types for Firecrawl API

#[derive(Serialize)]
struct ScrapeRequest {
    url: String,
    formats: Vec<String>,
}

#[derive(Deserialize)]
struct ScrapeResponse {
    success: bool,
    data: Option<ScrapeData>,
}

#[derive(Deserialize)]
struct ScrapeData {
    markdown: Option<String>,
    metadata: Option<PageMetadata>,
}

#[derive(Deserialize)]
struct PageMetadata {
    title: Option<String>,
    #[serde(rename = "sourceURL")]
    source_url: Option<String>,
}

#[derive(Serialize)]
struct CrawlRequest {
    url: String,
    limit: u32,
    #[serde(rename = "maxDepth")]
    max_depth: u32,
    #[serde(rename = "scrapeOptions")]
    scrape_options: CrawlScrapeOptions,
    #[serde(rename = "includePaths", skip_serializing_if = "Vec::is_empty")]
    include_paths: Vec<String>,
    #[serde(rename = "excludePaths", skip_serializing_if = "Vec::is_empty")]
    exclude_paths: Vec<String>,
}

#[derive(Serialize)]
struct CrawlScrapeOptions {
    formats: Vec<String>,
    #[serde(rename = "onlyMainContent")]
    only_main_content: bool,
}

#[derive(Deserialize)]
struct CrawlStartResponse {
    success: bool,
    id: Option<String>,
}

#[derive(Deserialize)]
struct CrawlStatusResponse {
    status: String,
    completed: Option<u32>,
    total: Option<u32>,
    data: Option<Vec<CrawlPageData>>,
}

#[derive(Deserialize)]
struct CrawlPageData {
    markdown: Option<String>,
    metadata: Option<PageMetadata>,
}

impl FirecrawlIngestor {
    /// Create a new Firecrawl ingestor with the given API key.
    pub fn new(api_key: impl Into<String>) -> CrawlResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        Ok(Self {
            client,
            api_key: api_key.into(),
            poll_timeout_secs: 300, // 5 minutes
            poll_interval_secs: 5,
        })
    }

    /// Create from environment variable `FIRECRAWL_API_KEY`.
    pub fn from_env() -> CrawlResult<Self> {
        let api_key = std::env::var("FIRECRAWL_API_KEY").map_err(|_| {
            CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "FIRECRAWL_API_KEY environment variable not set",
            )))
        })?;
        Self::new(api_key)
    }

    /// Set the poll timeout (seconds).
    pub fn with_poll_timeout(mut self, secs: u64) -> Self {
        self.poll_timeout_secs = secs;
        self
    }

    /// Set the poll interval (seconds).
    pub fn with_poll_interval(mut self, secs: u64) -> Self {
        self.poll_interval_secs = secs;
        self
    }

    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> CrawlResult<R> {
        let url = format!("{}{}", FIRECRAWL_API_URL, endpoint);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Firecrawl API error: {} - {}", status, text),
            ))));
        }

        response
            .json()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))
    }

    async fn get<R: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> CrawlResult<R> {
        let url = format!("{}{}", FIRECRAWL_API_URL, endpoint);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Firecrawl API error: {} - {}", status, text),
            ))));
        }

        response
            .json()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))
    }

    /// Scrape a single URL.
    async fn scrape_single(&self, url: &str) -> CrawlResult<RawPage> {
        let request = ScrapeRequest {
            url: url.to_string(),
            formats: vec!["markdown".to_string()],
        };

        let response: ScrapeResponse = self.post("/scrape", &request).await?;

        if !response.success {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Firecrawl scrape failed",
            ))));
        }

        let data = response.data.ok_or_else(|| {
            CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No data returned from Firecrawl",
            )))
        })?;

        let markdown = data.markdown.ok_or_else(|| {
            CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No markdown content returned from Firecrawl",
            )))
        })?;

        let mut page = RawPage::new(url, markdown)
            .with_fetched_at(Utc::now())
            .with_content_type("text/markdown")
            .with_metadata("source", "firecrawl");

        if let Some(title) = data.metadata.and_then(|m| m.title) {
            page = page.with_title(title);
        }

        Ok(page)
    }

    /// Convert Firecrawl page data to RawPage.
    fn page_data_to_raw_page(&self, data: CrawlPageData) -> Option<RawPage> {
        let markdown = data.markdown?;
        if markdown.trim().is_empty() {
            return None;
        }

        let url = data
            .metadata
            .as_ref()
            .and_then(|m| m.source_url.clone())
            .unwrap_or_default();

        if url.is_empty() {
            return None;
        }

        let mut page = RawPage::new(&url, markdown)
            .with_fetched_at(Utc::now())
            .with_content_type("text/markdown")
            .with_metadata("source", "firecrawl");

        if let Some(title) = data.metadata.and_then(|m| m.title) {
            page = page.with_title(title);
        }

        Some(page)
    }
}

#[async_trait]
impl Ingestor for FirecrawlIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        tracing::info!(
            url = %config.url,
            limit = %config.limit,
            max_depth = %config.max_depth,
            "Starting Firecrawl discover"
        );

        // Start the crawl
        let request = CrawlRequest {
            url: config.url.clone(),
            limit: config.limit as u32,
            max_depth: config.max_depth as u32,
            scrape_options: CrawlScrapeOptions {
                formats: vec!["markdown".to_string()],
                only_main_content: true,
            },
            include_paths: config.include_patterns.clone(),
            exclude_paths: config.exclude_patterns.clone(),
        };

        let start_response: CrawlStartResponse = self.post("/crawl", &request).await?;

        if !start_response.success {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to start Firecrawl crawl",
            ))));
        }

        let crawl_id = start_response.id.ok_or_else(|| {
            CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No crawl ID returned",
            )))
        })?;

        tracing::info!(crawl_id = %crawl_id, "Crawl started, polling for results");

        // Poll for completion
        let max_attempts = self.poll_timeout_secs / self.poll_interval_secs;
        let mut attempts = 0;

        loop {
            attempts += 1;
            if attempts > max_attempts {
                return Err(CrawlError::Timeout {
                    url: config.url.clone(),
                });
            }

            tokio::time::sleep(Duration::from_secs(self.poll_interval_secs)).await;

            let status: CrawlStatusResponse = self.get(&format!("/crawl/{}", crawl_id)).await?;

            match status.status.as_str() {
                "completed" => {
                    let pages: Vec<RawPage> = status
                        .data
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|data| self.page_data_to_raw_page(data))
                        .collect();

                    tracing::info!(
                        url = %config.url,
                        pages_discovered = pages.len(),
                        "Firecrawl discover completed"
                    );

                    return Ok(pages);
                }
                "failed" => {
                    return Err(CrawlError::Http(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Firecrawl crawl failed",
                    ))));
                }
                _ => {
                    // Still crawling, continue polling
                    if attempts % 6 == 0 {
                        tracing::info!(
                            crawl_id = %crawl_id,
                            status = %status.status,
                            completed = ?status.completed,
                            total = ?status.total,
                            "Crawl in progress"
                        );
                    }
                }
            }
        }
    }

    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        let mut pages = Vec::with_capacity(urls.len());

        for url in urls {
            match self.scrape_single(url).await {
                Ok(page) => pages.push(page),
                Err(e) => {
                    tracing::warn!("Failed to scrape {}: {}", url, e);
                }
            }
        }

        Ok(pages)
    }

    fn name(&self) -> &str {
        "firecrawl"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ingestor() {
        // This should succeed even without a valid API key (for construction)
        let ingestor = FirecrawlIngestor::new("test-key").unwrap();
        assert_eq!(ingestor.name(), "firecrawl");
    }

    #[test]
    fn test_page_data_to_raw_page() {
        let ingestor = FirecrawlIngestor::new("test-key").unwrap();

        let data = CrawlPageData {
            markdown: Some("# Test\n\nContent".to_string()),
            metadata: Some(PageMetadata {
                title: Some("Test Page".to_string()),
                source_url: Some("https://example.com/test".to_string()),
            }),
        };

        let page = ingestor.page_data_to_raw_page(data).unwrap();
        assert_eq!(page.url, "https://example.com/test");
        assert_eq!(page.title, Some("Test Page".to_string()));
        assert!(page.content.contains("# Test"));
    }

    #[test]
    fn test_page_data_to_raw_page_empty_content() {
        let ingestor = FirecrawlIngestor::new("test-key").unwrap();

        let data = CrawlPageData {
            markdown: Some("   ".to_string()),
            metadata: Some(PageMetadata {
                title: Some("Empty".to_string()),
                source_url: Some("https://example.com/empty".to_string()),
            }),
        };

        assert!(ingestor.page_data_to_raw_page(data).is_none());
    }

    #[test]
    fn test_page_data_to_raw_page_no_url() {
        let ingestor = FirecrawlIngestor::new("test-key").unwrap();

        let data = CrawlPageData {
            markdown: Some("Content".to_string()),
            metadata: None,
        };

        assert!(ingestor.page_data_to_raw_page(data).is_none());
    }
}
