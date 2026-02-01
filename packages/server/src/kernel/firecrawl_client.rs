//! Firecrawl API client using direct HTTP calls
//!
//! Replaces the firecrawl crate to avoid noisy println! debugging output.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

use super::{BaseWebScraper, CrawlResult, CrawledPage, LinkPriorities, ScrapeResult};

const FIRECRAWL_API_URL: &str = "https://api.firecrawl.dev/v1";

/// Firecrawl client using direct API calls
pub struct FirecrawlClient {
    client: Client,
    api_key: String,
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

impl FirecrawlClient {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, api_key })
    }

    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<R> {
        let url = format!("{}{}", FIRECRAWL_API_URL, endpoint);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .context("Failed to send request to Firecrawl")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Firecrawl API error: {} - {}", status, text);
        }

        response
            .json()
            .await
            .context("Failed to parse Firecrawl response")
    }

    async fn get<R: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<R> {
        let url = format!("{}{}", FIRECRAWL_API_URL, endpoint);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context("Failed to send request to Firecrawl")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Firecrawl API error: {} - {}", status, text);
        }

        response
            .json()
            .await
            .context("Failed to parse Firecrawl response")
    }
}

#[async_trait]
impl BaseWebScraper for FirecrawlClient {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        let request = ScrapeRequest {
            url: url.to_string(),
            formats: vec!["markdown".to_string()],
        };

        let response: ScrapeResponse = self.post("/scrape", &request).await?;

        if !response.success {
            anyhow::bail!("Firecrawl scrape failed");
        }

        let data = response.data.context("No data returned from Firecrawl")?;
        let markdown = data
            .markdown
            .context("No markdown content returned from Firecrawl")?;

        Ok(ScrapeResult {
            url: url.to_string(),
            markdown,
            title: data.metadata.and_then(|m| m.title),
        })
    }

    async fn crawl(
        &self,
        url: &str,
        max_depth: i32,
        max_pages: i32,
        _delay_seconds: i32,
        _priorities: Option<&LinkPriorities>,
    ) -> Result<CrawlResult> {
        info!(
            url = %url,
            max_depth = %max_depth,
            max_pages = %max_pages,
            "Starting Firecrawl crawl"
        );

        // Start the crawl
        let request = CrawlRequest {
            url: url.to_string(),
            limit: max_pages as u32,
            max_depth: max_depth as u32,
            scrape_options: CrawlScrapeOptions {
                formats: vec!["markdown".to_string()],
                only_main_content: true,
            },
        };

        let start_response: CrawlStartResponse = self.post("/crawl", &request).await?;

        if !start_response.success {
            anyhow::bail!("Failed to start Firecrawl crawl");
        }

        let crawl_id = start_response.id.context("No crawl ID returned")?;
        info!(crawl_id = %crawl_id, "Crawl started, polling for results");

        // Poll for completion
        let mut attempts = 0;
        let max_attempts = 60; // 5 minutes max (5s * 60)

        loop {
            attempts += 1;
            if attempts > max_attempts {
                anyhow::bail!("Crawl timed out after {} attempts", max_attempts);
            }

            tokio::time::sleep(Duration::from_secs(5)).await;

            let status: CrawlStatusResponse = self.get(&format!("/crawl/{}", crawl_id)).await?;

            match status.status.as_str() {
                "completed" => {
                    let pages = status
                        .data
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|page| {
                            let markdown = page.markdown?;
                            if markdown.trim().is_empty() {
                                return None;
                            }

                            Some(CrawledPage {
                                url: page
                                    .metadata
                                    .as_ref()
                                    .and_then(|m| m.source_url.clone())
                                    .unwrap_or_default(),
                                markdown,
                                title: page.metadata.and_then(|m| m.title),
                            })
                        })
                        .collect::<Vec<_>>();

                    info!(
                        url = %url,
                        pages_crawled = pages.len(),
                        "Firecrawl completed"
                    );

                    return Ok(CrawlResult { pages });
                }
                "failed" => {
                    anyhow::bail!("Firecrawl crawl failed");
                }
                _ => {
                    // Still crawling, continue polling
                    if attempts % 6 == 0 {
                        // Log progress every 30 seconds
                        info!(
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
}
