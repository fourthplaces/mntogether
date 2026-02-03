//! Informed crawler - query-driven page discovery.
//!
//! Combines standard crawling with search-based discovery to find
//! deep-linked pages that might have the answer.

use async_trait::async_trait;
use std::collections::HashSet;

use crate::error::CrawlResult;
use crate::traits::crawler::Crawler;
use crate::types::{config::CrawlConfig, page::CrawledPage};

/// Search service trait for discovering URLs.
#[async_trait]
pub trait SearchService: Send + Sync {
    /// Search for URLs on a specific site.
    async fn search_site(&self, site_url: &str, query: &str, limit: usize) -> Vec<String>;
}

/// Informed crawler that combines HTTP crawling with search-based discovery.
///
/// This addresses the "deep page" problem where relevant content might be
/// several clicks deep and missed by breadth-first crawling.
pub struct InformedCrawler<C: Crawler, S: SearchService> {
    http_crawler: C,
    search: S,
}

impl<C: Crawler, S: SearchService> InformedCrawler<C, S> {
    /// Create a new informed crawler.
    pub fn new(http_crawler: C, search: S) -> Self {
        Self {
            http_crawler,
            search,
        }
    }

    /// Crawl with query-driven discovery.
    ///
    /// # Arguments
    /// * `config` - Standard crawl configuration
    /// * `query` - The extraction query to inform discovery
    pub async fn crawl_for_query(
        &self,
        config: &CrawlConfig,
        query: &str,
    ) -> CrawlResult<Vec<CrawledPage>> {
        // 1. Search-based discovery (jump to relevant pages)
        let search_urls = self
            .search
            .search_site(&config.url, query, 20)
            .await;

        // 2. Standard crawl from root
        let crawled = self.http_crawler.crawl(config).await?;
        let crawled_urls: HashSet<_> = crawled.iter().map(|p| p.url.clone()).collect();

        // 3. Fetch search-discovered pages not in crawl
        let mut additional: Vec<CrawledPage> = Vec::new();
        for url in search_urls {
            if !crawled_urls.contains(&url) {
                match self.http_crawler.fetch(&url).await {
                    Ok(page) => additional.push(page),
                    Err(e) => {
                        tracing::warn!("Failed to fetch search result {}: {}", url, e);
                    }
                }
            }
        }

        // 4. Combine results
        let mut all_pages = crawled;
        all_pages.extend(additional);

        Ok(all_pages)
    }

    /// Deep crawl for filling gaps.
    ///
    /// When extraction returns gaps, this method searches for pages
    /// that might contain the missing information.
    pub async fn crawl_for_gap(
        &self,
        site_url: &str,
        gap_query: &str,
    ) -> CrawlResult<Vec<CrawledPage>> {
        // Search for pages that might have the answer
        let urls = self
            .search
            .search_site(site_url, gap_query, 10)
            .await;

        // Fetch those pages
        let mut pages: Vec<CrawledPage> = Vec::new();
        for url in urls {
            match self.http_crawler.fetch(&url).await {
                Ok(page) => pages.push(page),
                Err(e) => {
                    tracing::warn!("Failed to fetch gap page {}: {}", url, e);
                }
            }
        }

        Ok(pages)
    }
}

#[async_trait]
impl<C: Crawler, S: SearchService> Crawler for InformedCrawler<C, S> {
    async fn crawl(&self, config: &CrawlConfig) -> CrawlResult<Vec<CrawledPage>> {
        // Without a query, just do standard crawling
        self.http_crawler.crawl(config).await
    }

    async fn fetch(&self, url: &str) -> CrawlResult<CrawledPage> {
        self.http_crawler.fetch(url).await
    }
}

/// Simple search service using web search.
pub struct WebSearchService {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl WebSearchService {
    /// Create a new web search service.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for WebSearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchService for WebSearchService {
    async fn search_site(&self, site_url: &str, query: &str, limit: usize) -> Vec<String> {
        // This is a placeholder - in production, use a real search API
        // like Tavily, Bing, or Google Custom Search

        // Extract domain
        let domain = match url::Url::parse(site_url) {
            Ok(u) => u.host_str().unwrap_or("").to_string(),
            Err(_) => return vec![],
        };

        // Log the search intent (actual implementation would call an API)
        tracing::debug!(
            "Search: site:{} {} (limit: {})",
            domain,
            query,
            limit
        );

        // Return empty - subclasses should implement actual search
        vec![]
    }
}

/// Mock search service for testing.
#[derive(Default)]
pub struct MockSearchService {
    results: std::sync::RwLock<std::collections::HashMap<String, Vec<String>>>,
}

impl MockSearchService {
    /// Create a new mock search service.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add results for a query.
    pub fn with_results(self, query: &str, urls: Vec<String>) -> Self {
        self.results.write().unwrap().insert(query.to_string(), urls);
        self
    }
}

#[async_trait]
impl SearchService for MockSearchService {
    async fn search_site(&self, _site_url: &str, query: &str, limit: usize) -> Vec<String> {
        self.results
            .read()
            .unwrap()
            .get(query)
            .map(|urls| urls.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }
}

/// Tavily-backed search service.
pub struct TavilySearchService {
    api_key: crate::security::SecretString,
    client: reqwest::Client,
}

impl TavilySearchService {
    /// Create a new Tavily search service.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: crate::security::SecretString::new(api_key),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SearchService for TavilySearchService {
    async fn search_site(&self, site_url: &str, query: &str, limit: usize) -> Vec<String> {
        let domain = match url::Url::parse(site_url) {
            Ok(u) => u.host_str().unwrap_or("").to_string(),
            Err(_) => return vec![],
        };

        #[derive(serde::Serialize)]
        struct Request {
            query: String,
            search_depth: String,
            include_domains: Vec<String>,
            max_results: usize,
        }

        #[derive(serde::Deserialize)]
        struct Response {
            results: Vec<Result>,
        }

        #[derive(serde::Deserialize)]
        struct Result {
            url: String,
        }

        let request = Request {
            query: format!("site:{} {}", domain, query),
            search_depth: "basic".to_string(),
            include_domains: vec![domain],
            max_results: limit,
        };

        match self
            .client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .json(&request)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<Response>().await {
                    Ok(r) => r.results.into_iter().map(|r| r.url).collect(),
                    Err(_) => vec![],
                }
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockCrawler;

    #[tokio::test]
    async fn test_informed_crawler_combines_results() {
        let http_crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/", "Home"))
            .with_page(CrawledPage::new("https://example.com/about", "About"))
            .with_page(CrawledPage::new("https://example.com/deep/page", "Deep")); // From search

        let search = MockSearchService::new().with_results(
            "volunteer",
            vec!["https://example.com/deep/page".to_string()],
        );

        let crawler = InformedCrawler::new(http_crawler, search);

        let config = CrawlConfig::new("https://example.com");
        let pages = crawler.crawl_for_query(&config, "volunteer").await.unwrap();

        // Should include pages from both crawl and search
        assert!(pages.len() >= 2);
    }

    #[tokio::test]
    async fn test_crawl_for_gap() {
        let http_crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/contact", "Contact info"));

        let search = MockSearchService::new().with_results(
            "contact email",
            vec!["https://example.com/contact".to_string()],
        );

        let crawler = InformedCrawler::new(http_crawler, search);

        let pages = crawler
            .crawl_for_gap("https://example.com", "contact email")
            .await
            .unwrap();

        assert_eq!(pages.len(), 1);
        assert!(pages[0].content.contains("Contact"));
    }
}
