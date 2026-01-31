use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{BaseWebScraper, ScrapeResult};

/// Firecrawl client implementation of BaseWebScraper
pub struct FirecrawlClient {
    client: firecrawl::FirecrawlApp,
}

impl FirecrawlClient {
    pub fn new(api_key: String) -> Result<Self> {
        let client =
            firecrawl::FirecrawlApp::new(api_key).context("Failed to create Firecrawl client")?;

        Ok(Self { client })
    }
}

#[async_trait]
impl BaseWebScraper for FirecrawlClient {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        let result = self
            .client
            .scrape_url(
                url,
                Some(firecrawl::scrape::ScrapeOptions {
                    formats: Some(vec![firecrawl::scrape::ScrapeFormats::Markdown]),
                    ..Default::default()
                }),
            )
            .await
            .context("Firecrawl scrape failed")?;

        let markdown = result
            .markdown
            .context("No markdown content returned from Firecrawl")?;

        Ok(ScrapeResult {
            url: url.to_string(),
            markdown,
            title: result.metadata.title,
        })
    }
}
