//! Tavily-powered search crawler.
//!
//! Uses Tavily API to discover relevant pages on a site.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{CrawlError, CrawlResult};
use crate::security::SecretString;
use crate::traits::crawler::Crawler;
use crate::types::{config::CrawlConfig, page::CrawledPage};

/// Tavily search response.
#[derive(Debug, Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

/// A single Tavily search result.
#[derive(Debug, Deserialize)]
struct TavilyResult {
    url: String,
    title: String,
    content: String,
    #[allow(dead_code)]
    score: f64,
}

/// Tavily search request.
#[derive(Debug, Serialize)]
struct TavilyRequest {
    query: String,
    search_depth: String,
    include_domains: Vec<String>,
    max_results: usize,
}

/// Crawler that uses Tavily search to discover pages.
///
/// Better for sites where you need to find specific content
/// rather than crawling everything.
pub struct TavilyCrawler {
    client: reqwest::Client,
    api_key: SecretString,
    search_depth: String,
}

impl TavilyCrawler {
    /// Create a new Tavily crawler.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: SecretString::new(api_key),
            search_depth: "basic".to_string(),
        }
    }

    /// Set search depth ("basic" or "advanced").
    pub fn with_search_depth(mut self, depth: impl Into<String>) -> Self {
        self.search_depth = depth.into();
        self
    }

    /// Search for pages on a specific site.
    pub async fn search_site(
        &self,
        site_url: &str,
        query: &str,
        max_results: usize,
    ) -> CrawlResult<Vec<CrawledPage>> {
        // Extract domain from URL
        let domain = url::Url::parse(site_url)
            .map_err(|_| CrawlError::InvalidUrl {
                url: site_url.to_string(),
            })?
            .host_str()
            .ok_or_else(|| CrawlError::InvalidUrl {
                url: site_url.to_string(),
            })?
            .to_string();

        let request = TavilyRequest {
            query: format!("site:{} {}", domain, query),
            search_depth: self.search_depth.clone(),
            include_domains: vec![domain],
            max_results,
        };

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .json(&request)
            .send()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        if !response.status().is_success() {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Tavily API error: {}", response.status()),
            ))));
        }

        let tavily_response: TavilyResponse = response
            .json()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        Ok(tavily_response
            .results
            .into_iter()
            .map(|r| CrawledPage {
                url: r.url,
                content: r.content,
                title: Some(r.title),
                status_code: 200,
                headers: std::collections::HashMap::new(),
            })
            .collect())
    }

    /// Discover pages using multiple search queries.
    pub async fn discover(
        &self,
        site_url: &str,
        queries: &[&str],
        max_per_query: usize,
    ) -> CrawlResult<Vec<CrawledPage>> {
        let mut all_pages: Vec<CrawledPage> = Vec::new();
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

        for query in queries {
            match self.search_site(site_url, query, max_per_query).await {
                Ok(pages) => {
                    for page in pages {
                        if !seen_urls.contains(&page.url) {
                            seen_urls.insert(page.url.clone());
                            all_pages.push(page);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Tavily search failed for query '{}': {}", query, e);
                }
            }
        }

        Ok(all_pages)
    }
}

#[async_trait]
impl Crawler for TavilyCrawler {
    async fn crawl(&self, config: &CrawlConfig) -> CrawlResult<Vec<CrawledPage>> {
        // Use generic discovery queries
        let default_queries = [
            "site content",
            "services programs",
            "about contact",
            "volunteer donate",
            "events news",
        ];

        let max_per_query = config.max_pages / default_queries.len();

        self.discover(&config.url, &default_queries, max_per_query.max(5))
            .await
    }

    async fn fetch(&self, url: &str) -> CrawlResult<CrawledPage> {
        // Tavily doesn't support single-page fetch, use HTTP
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        if !response.status().is_success() {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP error: {}", response.status()),
            ))));
        }

        let content = response
            .text()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        Ok(CrawledPage {
            url: url.to_string(),
            content,
            title: None,
            status_code: 200,
            headers: std::collections::HashMap::new(),
        })
    }
}

/// Query generator for Tavily discovery.
pub struct QueryGenerator;

impl QueryGenerator {
    /// Generate discovery queries for a nonprofit site.
    pub fn nonprofit_queries() -> Vec<&'static str> {
        vec![
            "volunteer opportunities",
            "services programs",
            "donate donation",
            "events calendar",
            "contact about",
            "mission values",
            "resources help",
            "news updates",
        ]
    }

    /// Generate discovery queries for an e-commerce site.
    pub fn ecommerce_queries() -> Vec<&'static str> {
        vec![
            "products",
            "categories",
            "sale deals",
            "shipping returns",
            "contact support",
            "about us",
        ]
    }

    /// Generate discovery queries for a job board.
    pub fn job_board_queries() -> Vec<&'static str> {
        vec![
            "job listings careers",
            "apply application",
            "remote jobs",
            "full time part time",
            "salary benefits",
            "company culture",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a real Tavily API key
    // They are marked as ignored by default

    #[tokio::test]
    #[ignore]
    async fn test_tavily_search() {
        let api_key = std::env::var("TAVILY_API_KEY").expect("TAVILY_API_KEY required");
        let crawler = TavilyCrawler::new(api_key);

        let pages = crawler
            .search_site("https://example.com", "about", 5)
            .await
            .unwrap();

        assert!(!pages.is_empty());
    }
}
