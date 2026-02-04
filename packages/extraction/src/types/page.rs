//! Page types - cached pages and references.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// A cached page with content and metadata.
///
/// Pages are cached to avoid redundant crawls. The content hash
/// is used for cache invalidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPage {
    /// Canonical URL of the page
    pub url: String,

    /// Site URL this page belongs to (for filtering)
    pub site_url: String,

    /// Page content (usually markdown)
    pub content: String,

    /// SHA-256 hash of the content
    pub content_hash: String,

    /// When the page was fetched
    pub fetched_at: DateTime<Utc>,

    /// Page title if available
    pub title: Option<String>,

    /// HTTP headers from the response
    #[serde(default)]
    pub http_headers: HashMap<String, String>,

    /// Application-provided metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl CachedPage {
    /// Create a new cached page.
    pub fn new(
        url: impl Into<String>,
        site_url: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        let content_hash = Self::hash_content(&content);

        Self {
            url: url.into(),
            site_url: site_url.into(),
            content,
            content_hash,
            fetched_at: Utc::now(),
            title: None,
            http_headers: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Calculate SHA-256 hash of content.
    pub fn hash_content(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Set the page title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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

    /// Check if content has changed by comparing hashes.
    pub fn content_changed(&self, new_content: &str) -> bool {
        Self::hash_content(new_content) != self.content_hash
    }

    /// Check if the page is stale (older than threshold).
    pub fn is_stale(&self, max_age: chrono::Duration) -> bool {
        Utc::now() - self.fetched_at > max_age
    }

    /// Get content length in characters.
    pub fn content_length(&self) -> usize {
        self.content.len()
    }

    /// Extract the domain from the site URL.
    pub fn domain(&self) -> Option<&str> {
        url::Url::parse(&self.site_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .map(|_| self.site_url.as_str())
    }
}

/// A lightweight reference to a page (for search results).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRef {
    /// URL of the page
    pub url: String,

    /// Page title if available
    pub title: Option<String>,

    /// Site URL this page belongs to
    pub site_url: String,

    /// Relevance score (0.0 to 1.0)
    pub score: f32,
}

impl PageRef {
    /// Create a new page reference.
    pub fn new(url: impl Into<String>, site_url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: None,
            site_url: site_url.into(),
            score: 0.0,
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the relevance score.
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }
}

/// A page that was crawled (before caching).
#[derive(Debug, Clone)]
pub struct CrawledPage {
    /// URL that was crawled
    pub url: String,

    /// Raw content (usually HTML converted to markdown)
    pub content: String,

    /// Page title if available
    pub title: Option<String>,

    /// HTTP status code
    pub status_code: u16,

    /// HTTP headers
    pub headers: HashMap<String, String>,
}

impl CrawledPage {
    /// Create a new crawled page.
    pub fn new(url: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            content: content.into(),
            title: None,
            status_code: 200,
            headers: HashMap::new(),
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Convert to a CachedPage.
    pub fn into_cached(self, site_url: impl Into<String>) -> CachedPage {
        let mut page = CachedPage::new(self.url, site_url, self.content);
        page.title = self.title;
        page.http_headers = self.headers;
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let page = CachedPage::new(
            "https://example.com",
            "https://example.com",
            "Hello, world!",
        );
        assert!(!page.content_hash.is_empty());
        assert_eq!(page.content_hash.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_content_changed() {
        let page = CachedPage::new(
            "https://example.com",
            "https://example.com",
            "Hello, world!",
        );
        assert!(!page.content_changed("Hello, world!"));
        assert!(page.content_changed("Hello, universe!"));
    }

    #[test]
    fn test_is_stale() {
        let old_page = CachedPage::new("https://example.com", "https://example.com", "content")
            .with_fetched_at(Utc::now() - chrono::Duration::hours(25));

        assert!(old_page.is_stale(chrono::Duration::hours(24)));
        assert!(!old_page.is_stale(chrono::Duration::hours(48)));
    }
}
