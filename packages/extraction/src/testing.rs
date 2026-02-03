//! Testing utilities including mock implementations.
//!
//! These are useful for testing applications that use the extraction library
//! without making real AI or network calls.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::{CrawlError, CrawlResult, Result};
use crate::traits::{
    ai::{ExtractionStrategy, Partition, AI},
    crawler::Crawler,
};
use crate::types::{
    config::CrawlConfig,
    extraction::{Extraction, GroundingGrade, Source, SourceRole},
    page::{CachedPage, CrawledPage},
    summary::{RecallSignals, Summary, SummaryResponse},
};

/// A mock AI implementation for testing.
///
/// Returns deterministic, configurable responses for all AI operations.
/// Useful for testing extraction logic without making real LLM calls.
#[derive(Default)]
pub struct MockAI {
    /// Predefined summaries by URL
    summaries: Arc<RwLock<HashMap<String, SummaryResponse>>>,

    /// Predefined extractions by query
    extractions: Arc<RwLock<HashMap<String, Extraction>>>,

    /// Predefined query expansions
    expansions: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Predefined partitions by query
    partitions: Arc<RwLock<HashMap<String, Vec<Partition>>>>,

    /// Predefined embeddings by text
    embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,

    /// Default embedding dimension
    embedding_dim: usize,

    /// Strategy overrides
    strategy_overrides: Arc<RwLock<HashMap<String, ExtractionStrategy>>>,

    /// Call tracking for assertions
    calls: Arc<RwLock<Vec<MockAICall>>>,
}

/// Record of a call made to the mock AI.
#[derive(Debug, Clone)]
pub enum MockAICall {
    Summarize { url: String },
    ExpandQuery { query: String },
    ClassifyQuery { query: String },
    Partition { query: String, summary_count: usize },
    Extract { query: String, page_count: usize },
    Embed { text_len: usize },
}

impl MockAI {
    /// Create a new mock AI with default behavior.
    pub fn new() -> Self {
        Self {
            embedding_dim: 1024,
            ..Default::default()
        }
    }

    /// Set the embedding dimension.
    pub fn with_embedding_dim(mut self, dim: usize) -> Self {
        self.embedding_dim = dim;
        self
    }

    /// Add a predefined summary for a URL.
    pub fn with_summary(self, url: impl Into<String>, response: SummaryResponse) -> Self {
        self.summaries.write().unwrap().insert(url.into(), response);
        self
    }

    /// Add a predefined extraction for a query.
    pub fn with_extraction(self, query: impl Into<String>, extraction: Extraction) -> Self {
        self.extractions
            .write()
            .unwrap()
            .insert(query.into(), extraction);
        self
    }

    /// Add a predefined query expansion.
    pub fn with_expansion(self, query: impl Into<String>, terms: Vec<String>) -> Self {
        self.expansions.write().unwrap().insert(query.into(), terms);
        self
    }

    /// Add predefined partitions for a query.
    pub fn with_partitions(self, query: impl Into<String>, partitions: Vec<Partition>) -> Self {
        self.partitions
            .write()
            .unwrap()
            .insert(query.into(), partitions);
        self
    }

    /// Add a predefined embedding for text.
    pub fn with_embedding(self, text: impl Into<String>, embedding: Vec<f32>) -> Self {
        self.embeddings
            .write()
            .unwrap()
            .insert(text.into(), embedding);
        self
    }

    /// Override the strategy for a query.
    pub fn with_strategy(self, query: impl Into<String>, strategy: ExtractionStrategy) -> Self {
        self.strategy_overrides
            .write()
            .unwrap()
            .insert(query.into(), strategy);
        self
    }

    /// Get all calls made to this mock.
    pub fn calls(&self) -> Vec<MockAICall> {
        self.calls.read().unwrap().clone()
    }

    /// Clear call history.
    pub fn clear_calls(&self) {
        self.calls.write().unwrap().clear();
    }

    /// Generate a deterministic embedding based on text.
    fn generate_deterministic_embedding(&self, text: &str) -> Vec<f32> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();

        // Use hash bytes to seed a deterministic embedding
        (0..self.embedding_dim)
            .map(|i| {
                let byte_idx = i % 32;
                let byte = hash[byte_idx] as f32;
                // Normalize to [-1, 1] range
                (byte / 127.5) - 1.0
            })
            .collect()
    }

    /// Generate a default summary for unknown URLs.
    fn default_summary(&self, url: &str) -> SummaryResponse {
        SummaryResponse {
            summary: format!("Summary of page at {}", url),
            signals: RecallSignals::default(),
            language: Some("en".to_string()),
        }
    }

    /// Generate a default extraction for unknown queries.
    fn default_extraction(&self, query: &str, pages: &[CachedPage]) -> Extraction {
        let sources: Vec<Source> = pages
            .iter()
            .enumerate()
            .map(|(i, p)| Source {
                url: p.url.clone(),
                title: p.title.clone(),
                fetched_at: p.fetched_at,
                role: if i == 0 {
                    SourceRole::Primary
                } else {
                    SourceRole::Supporting
                },
                metadata: HashMap::new(),
            })
            .collect();

        let grounding = if sources.len() >= 2 {
            GroundingGrade::Verified
        } else if sources.len() == 1 {
            GroundingGrade::SingleSource
        } else {
            GroundingGrade::Inferred
        };

        Extraction {
            content: format!("Extracted content for query: {}", query),
            sources,
            gaps: vec![],
            grounding,
            conflicts: vec![],
        }
    }
}

#[async_trait]
impl AI for MockAI {
    async fn summarize(&self, _content: &str, url: &str) -> Result<SummaryResponse> {
        self.calls
            .write()
            .unwrap()
            .push(MockAICall::Summarize { url: url.to_string() });

        // Return predefined summary or generate default
        Ok(self
            .summaries
            .read()
            .unwrap()
            .get(url)
            .cloned()
            .unwrap_or_else(|| self.default_summary(url)))
    }

    async fn expand_query(&self, query: &str) -> Result<Vec<String>> {
        self.calls
            .write()
            .unwrap()
            .push(MockAICall::ExpandQuery {
                query: query.to_string(),
            });

        // Return predefined expansion or generate default
        Ok(self
            .expansions
            .read()
            .unwrap()
            .get(query)
            .cloned()
            .unwrap_or_else(|| {
                vec![
                    query.to_string(),
                    format!("{} related", query),
                    format!("about {}", query),
                ]
            }))
    }

    async fn classify_query(&self, query: &str) -> Result<ExtractionStrategy> {
        self.calls
            .write()
            .unwrap()
            .push(MockAICall::ClassifyQuery {
                query: query.to_string(),
            });

        // Return override or default to Collection
        Ok(self
            .strategy_overrides
            .read()
            .unwrap()
            .get(query)
            .copied()
            .unwrap_or(ExtractionStrategy::Collection))
    }

    async fn recall_and_partition(
        &self,
        query: &str,
        summaries: &[Summary],
    ) -> Result<Vec<Partition>> {
        self.calls
            .write()
            .unwrap()
            .push(MockAICall::Partition {
                query: query.to_string(),
                summary_count: summaries.len(),
            });

        // Return predefined partitions or generate default (one partition per summary)
        Ok(self
            .partitions
            .read()
            .unwrap()
            .get(query)
            .cloned()
            .unwrap_or_else(|| {
                summaries
                    .iter()
                    .map(|s| {
                        Partition::new(format!("Item from {}", s.url))
                            .with_url(&s.url)
                            .with_rationale("Default partition")
                    })
                    .collect()
            }))
    }

    async fn extract(
        &self,
        query: &str,
        pages: &[CachedPage],
        _hints: Option<&[String]>,
    ) -> Result<Extraction> {
        self.calls.write().unwrap().push(MockAICall::Extract {
            query: query.to_string(),
            page_count: pages.len(),
        });

        // Return predefined extraction or generate default
        Ok(self
            .extractions
            .read()
            .unwrap()
            .get(query)
            .cloned()
            .unwrap_or_else(|| self.default_extraction(query, pages)))
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.calls.write().unwrap().push(MockAICall::Embed {
            text_len: text.len(),
        });

        // Return predefined embedding or generate deterministic one
        Ok(self
            .embeddings
            .read()
            .unwrap()
            .get(text)
            .cloned()
            .unwrap_or_else(|| self.generate_deterministic_embedding(text)))
    }
}

/// A mock crawler for testing.
///
/// Returns predefined pages without making network requests.
#[derive(Default)]
pub struct MockCrawler {
    /// Predefined pages by URL
    pages: Arc<RwLock<HashMap<String, CrawledPage>>>,

    /// URLs that should fail
    fail_urls: Arc<RwLock<Vec<String>>>,

    /// Call tracking
    calls: Arc<RwLock<Vec<MockCrawlerCall>>>,
}

/// Record of a call made to the mock crawler.
#[derive(Debug, Clone)]
pub enum MockCrawlerCall {
    Crawl { url: String, max_pages: usize },
    Fetch { url: String },
}

impl MockCrawler {
    /// Create a new mock crawler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a predefined page.
    pub fn with_page(self, page: CrawledPage) -> Self {
        self.pages.write().unwrap().insert(page.url.clone(), page);
        self
    }

    /// Add multiple predefined pages.
    pub fn with_pages(self, pages: impl IntoIterator<Item = CrawledPage>) -> Self {
        let mut store = self.pages.write().unwrap();
        for page in pages {
            store.insert(page.url.clone(), page);
        }
        drop(store);
        self
    }

    /// Mark a URL as failing.
    pub fn fail_url(self, url: impl Into<String>) -> Self {
        self.fail_urls.write().unwrap().push(url.into());
        self
    }

    /// Get all calls made to this mock.
    pub fn calls(&self) -> Vec<MockCrawlerCall> {
        self.calls.read().unwrap().clone()
    }
}

#[async_trait]
impl Crawler for MockCrawler {
    async fn crawl(&self, config: &CrawlConfig) -> CrawlResult<Vec<CrawledPage>> {
        self.calls.write().unwrap().push(MockCrawlerCall::Crawl {
            url: config.url.clone(),
            max_pages: config.max_pages,
        });

        // Check if should fail
        if self.fail_urls.read().unwrap().contains(&config.url) {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "Mock connection refused",
            ))));
        }

        // Return pages matching the site URL prefix
        let pages: Vec<_> = self
            .pages
            .read()
            .unwrap()
            .values()
            .filter(|p| p.url.starts_with(&config.url))
            .take(config.max_pages)
            .cloned()
            .collect();

        Ok(pages)
    }

    async fn fetch(&self, url: &str) -> CrawlResult<CrawledPage> {
        self.calls
            .write()
            .unwrap()
            .push(MockCrawlerCall::Fetch { url: url.to_string() });

        // Check if should fail
        if self.fail_urls.read().unwrap().contains(&url.to_string()) {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "Mock connection refused",
            ))));
        }

        // Return predefined page or error
        self.pages
            .read()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| CrawlError::InvalidUrl { url: url.to_string() })
    }
}

/// Builder for creating test scenarios.
pub struct TestScenario {
    ai: MockAI,
    crawler: MockCrawler,
}

impl TestScenario {
    /// Create a new test scenario.
    pub fn new() -> Self {
        Self {
            ai: MockAI::new(),
            crawler: MockCrawler::new(),
        }
    }

    /// Add a site with pages.
    pub fn with_site(mut self, site_url: &str, pages: Vec<(&str, &str)>) -> Self {
        for (path, content) in pages {
            let url = format!("{}{}", site_url, path);
            let page = CrawledPage::new(&url, content).with_title(path);
            self.crawler = self.crawler.with_page(page);
        }
        self
    }

    /// Get the mock AI.
    pub fn ai(self) -> MockAI {
        self.ai
    }

    /// Get the mock crawler.
    pub fn crawler(self) -> MockCrawler {
        self.crawler
    }

    /// Get both mocks.
    pub fn build(self) -> (MockAI, MockCrawler) {
        (self.ai, self.crawler)
    }
}

impl Default for TestScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_ai_summarize() {
        let ai = MockAI::new().with_summary(
            "https://example.com",
            SummaryResponse {
                summary: "Test summary".to_string(),
                signals: RecallSignals::default(),
                language: Some("en".to_string()),
            },
        );

        let result = ai.summarize("content", "https://example.com").await.unwrap();
        assert_eq!(result.summary, "Test summary");

        // Check call was recorded
        let calls = ai.calls();
        assert_eq!(calls.len(), 1);
        assert!(matches!(calls[0], MockAICall::Summarize { .. }));
    }

    #[tokio::test]
    async fn test_mock_ai_embed_deterministic() {
        let ai = MockAI::new().with_embedding_dim(128);

        let emb1 = ai.embed("hello").await.unwrap();
        let emb2 = ai.embed("hello").await.unwrap();
        let emb3 = ai.embed("world").await.unwrap();

        assert_eq!(emb1.len(), 128);
        assert_eq!(emb1, emb2); // Same input = same output
        assert_ne!(emb1, emb3); // Different input = different output
    }

    #[tokio::test]
    async fn test_mock_crawler_fetch() {
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/page1", "Content 1"))
            .with_page(CrawledPage::new("https://example.com/page2", "Content 2"));

        let page = crawler.fetch("https://example.com/page1").await.unwrap();
        assert_eq!(page.content, "Content 1");

        // Non-existent page should fail
        let result = crawler.fetch("https://example.com/missing").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_crawler_fail_url() {
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/page1", "Content"))
            .fail_url("https://fail.com");

        let result = crawler.fetch("https://fail.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_test_scenario() {
        let (ai, crawler) = TestScenario::new()
            .with_site(
                "https://nonprofit.org",
                vec![
                    ("/", "Home page content"),
                    ("/volunteer", "Volunteer opportunities"),
                    ("/donate", "Donation information"),
                ],
            )
            .build();

        // Crawler should have the pages
        let page = crawler.fetch("https://nonprofit.org/volunteer").await.unwrap();
        assert!(page.content.contains("Volunteer"));

        // AI should work with defaults
        let summary = ai.summarize("content", "https://nonprofit.org").await.unwrap();
        assert!(!summary.summary.is_empty());
    }
}
