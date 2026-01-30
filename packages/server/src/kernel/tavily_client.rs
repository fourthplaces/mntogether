use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{BaseSearchService, SearchResult};

/// Tavily API client for web search
pub struct TavilyClient {
    api_key: String,
    client: reqwest::Client,
}

/// Tavily search depth
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
enum SearchDepth {
    Basic,
    Advanced,
}

/// Tavily API request
#[derive(Debug, Serialize)]
struct TavilyRequest {
    api_key: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_depth: Option<SearchDepth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    days: Option<i32>,
}

/// Tavily API response
#[derive(Debug, Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

/// Individual search result from Tavily
#[derive(Debug, Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
    score: f64,
    #[serde(default)]
    published_date: Option<String>,
}

impl TavilyClient {
    /// Create a new Tavily client
    pub fn new(api_key: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { api_key, client })
    }
}

#[async_trait]
impl BaseSearchService for TavilyClient {
    async fn search(
        &self,
        query: &str,
        max_results: Option<usize>,
        search_depth: Option<&str>,
        days: Option<i32>,
    ) -> Result<Vec<SearchResult>> {
        let depth = match search_depth {
            Some("advanced") => Some(SearchDepth::Advanced),
            Some("basic") | None => Some(SearchDepth::Basic),
            _ => Some(SearchDepth::Basic),
        };

        let request = TavilyRequest {
            api_key: self.api_key.clone(),
            query: query.to_string(),
            search_depth: depth,
            max_results,
            days,
        };

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .json(&request)
            .send()
            .await
            .context("Failed to send Tavily search request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Tavily API error {}: {}", status, body);
        }

        let tavily_response: TavilyResponse = response
            .json()
            .await
            .context("Failed to parse Tavily response")?;

        let results = tavily_response
            .results
            .into_iter()
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                content: r.content,
                score: r.score,
                published_date: r.published_date,
            })
            .collect();

        Ok(results)
    }
}

/// No-op search service for testing or when API key not configured
pub struct NoopSearchService;

#[async_trait]
impl BaseSearchService for NoopSearchService {
    async fn search(
        &self,
        _query: &str,
        _max_results: Option<usize>,
        _search_depth: Option<&str>,
        _days: Option<i32>,
    ) -> Result<Vec<SearchResult>> {
        tracing::warn!("NoopSearchService: search called but no Tavily API key configured");
        Ok(vec![])
    }
}
