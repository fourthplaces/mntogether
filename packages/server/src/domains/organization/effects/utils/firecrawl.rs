use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Firecrawl API client for scraping websites
pub struct FirecrawlClient {
    api_key: String,
    http_client: Client,
    base_url: String,
}

impl FirecrawlClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: Client::new(),
            base_url: "https://api.firecrawl.dev/v1".to_string(),
        }
    }

    /// Scrape a website and return clean text content
    pub async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        let request_body = ScrapeRequest {
            url: url.to_string(),
            formats: vec!["markdown".to_string()],
        };

        let response = self
            .http_client
            .post(&format!("{}/scrape", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send Firecrawl API request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Firecrawl API error (status {}): {}", status, error_text);
        }

        let api_response: FirecrawlResponse = response
            .json()
            .await
            .context("Failed to parse Firecrawl API response")?;

        Ok(ScrapeResult {
            url: api_response.data.url,
            markdown: api_response.data.markdown,
            title: api_response.data.metadata.title,
        })
    }
}

#[derive(Debug, Serialize)]
struct ScrapeRequest {
    url: String,
    formats: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FirecrawlResponse {
    success: bool,
    data: FirecrawlData,
}

#[derive(Debug, Deserialize)]
struct FirecrawlData {
    url: String,
    markdown: String,
    metadata: FirecrawlMetadata,
}

#[derive(Debug, Deserialize)]
struct FirecrawlMetadata {
    title: Option<String>,
}

/// Result of scraping a website
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub url: String,
    pub markdown: String,
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_scrape() {
        let api_key = std::env::var("FIRECRAWL_API_KEY")
            .expect("FIRECRAWL_API_KEY must be set for integration tests");

        let client = FirecrawlClient::new(api_key);

        let result = client
            .scrape("https://www.ascensionburnsville.org/")
            .await
            .expect("Scraping should succeed");

        assert!(!result.markdown.is_empty());
        println!("Scraped {} characters", result.markdown.len());
    }
}
