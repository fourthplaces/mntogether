use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::info;

use super::{BaseWebScraper, CrawlResult, CrawledPage, ScrapeResult};

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

    async fn crawl(
        &self,
        url: &str,
        max_depth: i32,
        max_pages: i32,
        _delay_seconds: i32,
    ) -> Result<CrawlResult> {
        info!(
            url = %url,
            max_depth = %max_depth,
            max_pages = %max_pages,
            "Starting multi-page crawl"
        );

        let options = firecrawl::crawl::CrawlOptions {
            max_depth: Some(max_depth as u32),
            limit: Some(max_pages as u32),
            scrape_options: Some(firecrawl::crawl::CrawlScrapeOptions {
                formats: Some(vec![firecrawl::crawl::CrawlScrapeFormats::Markdown]),
                only_main_content: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = self
            .client
            .crawl_url(url, Some(options))
            .await
            .context("Firecrawl crawl failed")?;

        info!(
            url = %url,
            pages_crawled = %result.data.len(),
            "Crawl completed"
        );

        let pages = result
            .data
            .into_iter()
            .filter_map(|doc| {
                // Skip pages with no markdown content
                let markdown = doc.markdown?;
                if markdown.trim().is_empty() {
                    return None;
                }

                Some(CrawledPage {
                    url: doc.metadata.source_url,
                    markdown,
                    title: doc.metadata.title,
                })
            })
            .collect();

        Ok(CrawlResult { pages })
    }
}
