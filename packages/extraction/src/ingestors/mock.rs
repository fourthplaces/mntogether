//! Mock ingestor for testing.
//!
//! Provides a configurable mock implementation of the Ingestor trait.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::CrawlResult;
use crate::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};

/// Mock ingestor for testing.
///
/// Allows configuring canned responses for discover and fetch operations.
///
/// # Example
///
/// ```rust
/// use extraction::ingestors::{MockIngestor, RawPage};
///
/// let mut mock = MockIngestor::new();
/// mock.add_page(RawPage::new("https://example.com", "# Hello\n\nWorld"));
///
/// // Now discover or fetch will return this page
/// ```
#[derive(Default)]
pub struct MockIngestor {
    /// Canned pages indexed by URL
    pages: Arc<RwLock<HashMap<String, RawPage>>>,
    /// Track call counts for verification
    discover_calls: Arc<RwLock<Vec<String>>>,
    fetch_calls: Arc<RwLock<Vec<Vec<String>>>>,
}

impl MockIngestor {
    /// Create a new empty mock ingestor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a page that will be returned by discover/fetch.
    pub fn add_page(&self, page: RawPage) {
        let mut pages = self.pages.write().unwrap();
        pages.insert(page.url.clone(), page);
    }

    /// Add multiple pages.
    pub fn add_pages(&self, pages: Vec<RawPage>) {
        for page in pages {
            self.add_page(page);
        }
    }

    /// Create a mock with pre-configured pages (builder pattern).
    pub fn with_page(self, page: RawPage) -> Self {
        self.add_page(page);
        self
    }

    /// Create a mock with multiple pre-configured pages (builder pattern).
    pub fn with_pages(self, pages: Vec<RawPage>) -> Self {
        self.add_pages(pages);
        self
    }

    /// Get the number of times discover was called.
    pub fn discover_call_count(&self) -> usize {
        self.discover_calls.read().unwrap().len()
    }

    /// Get the URLs that were requested via discover.
    pub fn discover_calls(&self) -> Vec<String> {
        self.discover_calls.read().unwrap().clone()
    }

    /// Get the number of times fetch_specific was called.
    pub fn fetch_call_count(&self) -> usize {
        self.fetch_calls.read().unwrap().len()
    }

    /// Get the URLs that were requested via fetch_specific.
    pub fn fetch_calls(&self) -> Vec<Vec<String>> {
        self.fetch_calls.read().unwrap().clone()
    }

    /// Clear all recorded calls.
    pub fn reset_calls(&self) {
        self.discover_calls.write().unwrap().clear();
        self.fetch_calls.write().unwrap().clear();
    }

    /// Clear all pages and calls.
    pub fn reset(&self) {
        self.pages.write().unwrap().clear();
        self.reset_calls();
    }
}

impl Clone for MockIngestor {
    fn clone(&self) -> Self {
        Self {
            pages: Arc::clone(&self.pages),
            discover_calls: Arc::clone(&self.discover_calls),
            fetch_calls: Arc::clone(&self.fetch_calls),
        }
    }
}

#[async_trait]
impl Ingestor for MockIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        // Record the call
        self.discover_calls
            .write()
            .unwrap()
            .push(config.url.clone());

        // Return all pages that match the site, up to limit
        let pages = self.pages.read().unwrap();
        let site_url = url::Url::parse(&config.url)
            .ok()
            .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or("")));

        let matching: Vec<RawPage> = pages
            .values()
            .filter(|p| {
                // Match pages from the same site
                if let Some(ref site) = site_url {
                    p.url.starts_with(site)
                } else {
                    true
                }
            })
            .take(config.limit)
            .cloned()
            .collect();

        Ok(matching)
    }

    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        // Record the call
        self.fetch_calls.write().unwrap().push(urls.to_vec());

        // Return pages that match the requested URLs
        let pages = self.pages.read().unwrap();
        let found: Vec<RawPage> = urls
            .iter()
            .filter_map(|url| pages.get(url).cloned())
            .collect();

        Ok(found)
    }

    fn name(&self) -> &str {
        "mock"
    }
}

/// Builder for creating test scenarios with the mock ingestor.
pub struct MockIngestorBuilder {
    mock: MockIngestor,
}

impl MockIngestorBuilder {
    /// Start building a mock ingestor.
    pub fn new() -> Self {
        Self {
            mock: MockIngestor::new(),
        }
    }

    /// Add a simple page with just URL and content.
    pub fn page(self, url: &str, content: &str) -> Self {
        self.mock.add_page(RawPage::new(url, content));
        self
    }

    /// Add a page with a title.
    pub fn page_with_title(self, url: &str, title: &str, content: &str) -> Self {
        self.mock
            .add_page(RawPage::new(url, content).with_title(title));
        self
    }

    /// Build the mock ingestor.
    pub fn build(self) -> MockIngestor {
        self.mock
    }
}

impl Default for MockIngestorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_discover() {
        let mock = MockIngestorBuilder::new()
            .page("https://example.com/", "# Home")
            .page("https://example.com/about", "# About")
            .page("https://other.com/", "# Other")
            .build();

        let config = DiscoverConfig::new("https://example.com").with_limit(10);
        let pages = mock.discover(&config).await.unwrap();

        // Should only return pages from example.com
        assert_eq!(pages.len(), 2);
        assert!(pages
            .iter()
            .all(|p| p.url.starts_with("https://example.com")));
    }

    #[tokio::test]
    async fn test_mock_fetch_specific() {
        let mock = MockIngestorBuilder::new()
            .page("https://example.com/a", "Page A")
            .page("https://example.com/b", "Page B")
            .page("https://example.com/c", "Page C")
            .build();

        let pages = mock
            .fetch_specific(&[
                "https://example.com/a".to_string(),
                "https://example.com/c".to_string(),
                "https://example.com/missing".to_string(),
            ])
            .await
            .unwrap();

        // Should return only found pages
        assert_eq!(pages.len(), 2);
        assert!(pages.iter().any(|p| p.url == "https://example.com/a"));
        assert!(pages.iter().any(|p| p.url == "https://example.com/c"));
    }

    #[tokio::test]
    async fn test_mock_call_tracking() {
        let mock = MockIngestor::new();

        let config = DiscoverConfig::new("https://example.com");
        mock.discover(&config).await.unwrap();
        mock.discover(&config).await.unwrap();

        assert_eq!(mock.discover_call_count(), 2);
        assert_eq!(
            mock.discover_calls(),
            vec![
                "https://example.com".to_string(),
                "https://example.com".to_string(),
            ]
        );

        mock.fetch_specific(&["https://example.com/a".to_string()])
            .await
            .unwrap();
        assert_eq!(mock.fetch_call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_discover_limit() {
        let mock = MockIngestor::new();
        for i in 0..100 {
            mock.add_page(RawPage::new(
                format!("https://example.com/{}", i),
                format!("Page {}", i),
            ));
        }

        let config = DiscoverConfig::new("https://example.com").with_limit(5);
        let pages = mock.discover(&config).await.unwrap();

        assert_eq!(pages.len(), 5);
    }
}
