use anyhow::{Context, Result};
use async_trait::async_trait;
use firecrawl::FirecrawlApp;

use crate::kernel::{BaseWebScraper, ScrapeResult};

/// Firecrawl API client for scraping websites (using official SDK)
pub struct FirecrawlClient {
    client: FirecrawlApp,
}

impl FirecrawlClient {
    pub fn new(api_key: String) -> Result<Self> {
        let client = FirecrawlApp::new(&api_key)
            .context("Failed to create Firecrawl client - check API key format")?;
        Ok(Self { client })
    }
}

#[async_trait]
impl BaseWebScraper for FirecrawlClient {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        // Limit to 15 pages to avoid exceeding Claude's 200k token context window
        // At ~5k tokens/page, 15 pages = ~75k tokens (safe buffer for prompt + response)
        let params = firecrawl::crawl::CrawlOptions {
            limit: Some(15),
            scrape_options: Some(firecrawl::crawl::CrawlScrapeOptions {
                formats: Some(vec![firecrawl::crawl::CrawlScrapeFormats::Markdown]),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Use crawl_url to spider the site (limited pages)
        let crawl_result = self
            .client
            .crawl_url(url, Some(params))
            .await
            .context("Failed to crawl site with Firecrawl SDK")?;

        tracing::info!(
            url = %url,
            pages_count = crawl_result.data.len(),
            "Firecrawl SDK crawl completed"
        );

        // Combine markdown from all crawled pages
        let mut combined_markdown = String::new();
        let mut main_title = String::from("Untitled");

        for (idx, page) in crawl_result.data.iter().enumerate() {
            // Use first page title as main title
            if idx == 0 {
                if let Some(title) = &page.metadata.title {
                    main_title = title.clone();
                }
            }

            // Add page markdown with source URL
            if let Some(markdown) = &page.markdown {
                combined_markdown.push_str(&format!(
                    "\n\n--- Page {}: {} ---\n\n{}",
                    idx + 1,
                    &page.metadata.source_url,
                    markdown
                ));
            }
        }

        tracing::info!(
            url = %url,
            combined_length = combined_markdown.len(),
            pages_crawled = crawl_result.data.len(),
            "Combined markdown from all pages"
        );

        Ok(ScrapeResult {
            url: url.to_string(),
            markdown: combined_markdown,
            title: Some(main_title),
        })
    }
}

// Old manual structs removed - using official SDK now

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_scrape() {
        let api_key = std::env::var("FIRECRAWL_API_KEY")
            .expect("FIRECRAWL_API_KEY must be set for integration tests");

        let client = FirecrawlClient::new(api_key)
            .expect("Failed to create Firecrawl client");

        let result = client
            .scrape("https://www.ascensionburnsville.org/")
            .await
            .expect("Scraping should succeed");

        assert!(!result.markdown.is_empty());
        println!("Scraped {} characters", result.markdown.len());
    }
}
