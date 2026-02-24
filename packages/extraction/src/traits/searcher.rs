//! Web searcher trait for external discovery.
//!
//! When the `PageStore` comes up empty, the library needs a way to discover
//! new relevant URLs. This trait abstracts over search providers (Tavily,
//! SerpAPI, Google Custom Search, etc.).
//!
//! # Design: "Push" vs "Pull"
//!
//! The library uses a "push" model: it returns `gaps`, and the **app** decides
//! whether to spend money on external search. This keeps the library as a
//! mechanical engine while the app remains the strategist.
//!
//! ```rust,ignore
//! // App decides when to use external search
//! let result = index.extract("volunteer contact info").await?;
//!
//! if !result.gaps.is_empty() {
//!     // App pays for search
//!     let urls = searcher.search(&result.gaps[0].query).await?;
//!
//!     // Enrich the index
//!     for url in urls {
//!         index.ingest_single_page(&url, &crawler).await?;
//!     }
//!
//!     // Re-extract with enriched index
//!     result = index.extract("volunteer contact info").await?;
//! }
//! ```

use async_trait::async_trait;
use url::Url;

use crate::error::Result;

/// A discovered URL from web search with metadata.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The discovered URL.
    pub url: Url,

    /// Title of the page (if available from search results).
    pub title: Option<String>,

    /// Snippet/description from search results.
    pub snippet: Option<String>,

    /// Relevance score (0.0-1.0, if provided by search API).
    pub score: Option<f32>,
}

impl SearchResult {
    /// Create a new search result from a URL.
    pub fn new(url: Url) -> Self {
        Self {
            url,
            title: None,
            snippet: None,
            score: None,
        }
    }

    /// Create from a URL string.
    pub fn from_url(url: &str) -> Option<Self> {
        Url::parse(url).ok().map(Self::new)
    }

    /// Add a title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a snippet.
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    /// Add a relevance score.
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }
}

/// Web search trait for open-world discovery.
///
/// Unlike `SearchService` (which is site-scoped), this trait searches the
/// entire web to discover new URLs that might contain the answer.
///
/// # Implementations
///
/// - `TavilyWebSearcher` - Tavily API
/// - `SerpWebSearcher` - SerpAPI (Google, Bing, etc.)
/// - `MockWebSearcher` - For testing
///
/// # Example
///
/// ```rust,ignore
/// let searcher = TavilyWebSearcher::new(api_key);
///
/// // Search for pages about a specific topic
/// let results = searcher.search("Red Cross volunteer coordinator email").await?;
///
/// for result in results {
///     println!("Found: {} - {:?}", result.url, result.title);
/// }
/// ```
#[async_trait]
pub trait WebSearcher: Send + Sync {
    /// Search the web for URLs relevant to the query.
    ///
    /// Returns discovered URLs that might contain the answer.
    /// The caller decides whether to crawl and ingest these pages.
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;

    /// Search with a specific result limit.
    async fn search_with_limit(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut results = self.search(query).await?;
        results.truncate(limit);
        Ok(results)
    }
}

/// Mock web searcher for testing.
#[derive(Default)]
pub struct MockWebSearcher {
    results: std::sync::RwLock<std::collections::HashMap<String, Vec<SearchResult>>>,
}

impl MockWebSearcher {
    /// Create a new mock searcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add results for a query.
    pub fn with_results(self, query: &str, results: Vec<SearchResult>) -> Self {
        self.results
            .write()
            .unwrap()
            .insert(query.to_string(), results);
        self
    }

    /// Add URL strings as results.
    pub fn with_urls(self, query: &str, urls: &[&str]) -> Self {
        let results: Vec<_> = urls
            .iter()
            .filter_map(|u| SearchResult::from_url(u))
            .collect();
        self.with_results(query, results)
    }
}

#[async_trait]
impl WebSearcher for MockWebSearcher {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        Ok(self
            .results
            .read()
            .unwrap()
            .get(query)
            .cloned()
            .unwrap_or_default())
    }
}

/// Tavily-backed web searcher.
///
/// Uses Tavily's search API for open-world URL discovery.
pub struct TavilyWebSearcher {
    api_key: crate::security::SecretString,
    client: reqwest::Client,
    /// Default number of results to return.
    pub default_limit: usize,
}

impl TavilyWebSearcher {
    /// Create a new Tavily web searcher.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: crate::security::SecretString::new(api_key),
            client: reqwest::Client::new(),
            default_limit: 10,
        }
    }

    /// Set the default result limit.
    pub fn with_default_limit(mut self, limit: usize) -> Self {
        self.default_limit = limit;
        self
    }
}

#[async_trait]
impl WebSearcher for TavilyWebSearcher {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        self.search_with_limit(query, self.default_limit).await
    }

    async fn search_with_limit(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        #[derive(serde::Serialize)]
        struct Request {
            query: String,
            search_depth: String,
            max_results: usize,
        }

        #[derive(serde::Deserialize)]
        struct Response {
            results: Vec<TavilyResult>,
        }

        #[derive(serde::Deserialize)]
        struct TavilyResult {
            url: String,
            title: Option<String>,
            content: Option<String>,
            score: Option<f32>,
        }

        let request = Request {
            query: query.to_string(),
            search_depth: "basic".to_string(),
            max_results: limit,
        };

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                crate::error::ExtractionError::Crawl(crate::error::CrawlError::Http(Box::new(e)))
            })?;

        if !response.status().is_success() {
            return Err(crate::error::ExtractionError::Crawl(
                crate::error::CrawlError::Http(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Tavily API error: {}", response.status()),
                ))),
            ));
        }

        let tavily_response: Response = response.json().await.map_err(|e| {
            crate::error::ExtractionError::Crawl(crate::error::CrawlError::Http(Box::new(e)))
        })?;

        let results = tavily_response
            .results
            .into_iter()
            .filter_map(|r| {
                let url = Url::parse(&r.url).ok()?;
                let mut result = SearchResult::new(url);
                if let Some(title) = r.title {
                    result = result.with_title(title);
                }
                if let Some(content) = r.content {
                    result = result.with_snippet(content);
                }
                if let Some(score) = r.score {
                    result = result.with_score(score);
                }
                Some(result)
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_web_searcher() {
        let searcher = MockWebSearcher::new().with_urls(
            "volunteer coordinator email",
            &[
                "https://redcross.org/volunteer",
                "https://redcross.org/contact",
            ],
        );

        let results = searcher
            .search("volunteer coordinator email")
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].url.as_str(), "https://redcross.org/volunteer");
    }

    #[tokio::test]
    async fn test_search_with_limit() {
        let searcher = MockWebSearcher::new().with_urls(
            "query",
            &[
                "https://a.com",
                "https://b.com",
                "https://c.com",
                "https://d.com",
            ],
        );

        let results = searcher.search_with_limit("query", 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
