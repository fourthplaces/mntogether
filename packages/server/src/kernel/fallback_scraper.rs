//! Fallback web scraper - tries simple scraper first, falls back to Firecrawl on 403
//!
//! This provides the best of both worlds:
//! - Free scraping for cooperative sites (SimpleScraper)
//! - Paid fallback for protected sites (Firecrawl)

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{info, warn};

use super::firecrawl_client::FirecrawlClient;
use super::simple_scraper::SimpleScraper;
use super::{BaseWebScraper, CrawlResult, LinkPriorities, ScrapeResult};

/// Scraper that tries SimpleScraper first, falls back to Firecrawl on 403/blocking
pub struct FallbackScraper {
    simple: SimpleScraper,
    firecrawl: Option<FirecrawlClient>,
}

impl FallbackScraper {
    /// Create a new FallbackScraper
    ///
    /// If firecrawl_api_key is None, 403 fallback is disabled
    pub fn new(firecrawl_api_key: Option<String>) -> Result<Self> {
        let simple = SimpleScraper::new()?;

        let firecrawl = match firecrawl_api_key {
            Some(key) if !key.is_empty() => {
                info!("Firecrawl fallback enabled for 403 errors");
                Some(FirecrawlClient::new(key)?)
            }
            _ => {
                info!("Firecrawl fallback disabled (no API key)");
                None
            }
        };

        Ok(Self { simple, firecrawl })
    }

    /// Check if an error indicates the site is blocking us
    fn is_blocking_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        error_str.contains("403")
            || error_str.contains("forbidden")
            || error_str.contains("access denied")
            || error_str.contains("blocked")
            || error_str.contains("cloudflare")
    }
}

#[async_trait]
impl BaseWebScraper for FallbackScraper {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        // Try simple scraper first
        match self.simple.scrape(url).await {
            Ok(result) => Ok(result),
            Err(e) if Self::is_blocking_error(&e) => {
                // Site is blocking us, try Firecrawl
                if let Some(ref firecrawl) = self.firecrawl {
                    warn!(
                        url = %url,
                        error = %e,
                        "Simple scraper blocked, falling back to Firecrawl"
                    );
                    firecrawl
                        .scrape(url)
                        .await
                        .context("Firecrawl fallback failed")
                } else {
                    warn!(
                        url = %url,
                        "Site blocking requests but Firecrawl not configured"
                    );
                    Err(e)
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn crawl(
        &self,
        url: &str,
        max_depth: i32,
        max_pages: i32,
        delay_seconds: i32,
        priorities: Option<&LinkPriorities>,
    ) -> Result<CrawlResult> {
        // Try simple scraper first
        match self
            .simple
            .crawl(url, max_depth, max_pages, delay_seconds, priorities)
            .await
        {
            Ok(result) if !result.pages.is_empty() => Ok(result),
            Ok(_) | Err(_) => {
                // No pages found or error - might be blocked, try Firecrawl
                if let Some(ref firecrawl) = self.firecrawl {
                    warn!(
                        url = %url,
                        "Simple crawl returned no pages, falling back to Firecrawl"
                    );
                    // Note: Firecrawl crawl doesn't use priorities
                    firecrawl
                        .crawl(url, max_depth, max_pages, delay_seconds, None)
                        .await
                        .context("Firecrawl crawl fallback failed")
                } else {
                    // Return empty result if no Firecrawl
                    Ok(CrawlResult { pages: vec![] })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blocking_error() {
        let err_403 = anyhow::anyhow!("HTTP 403 for https://example.com");
        assert!(FallbackScraper::is_blocking_error(&err_403));

        let err_forbidden = anyhow::anyhow!("Forbidden");
        assert!(FallbackScraper::is_blocking_error(&err_forbidden));

        let err_500 = anyhow::anyhow!("HTTP 500 Server Error");
        assert!(!FallbackScraper::is_blocking_error(&err_500));
    }
}
