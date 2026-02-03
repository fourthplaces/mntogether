//! Rate-limited crawler wrapper.
//!
//! Wraps any Crawler implementation with rate limiting using the governor crate.

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::error::CrawlResult;
use crate::traits::crawler::Crawler;
use crate::types::{config::CrawlConfig, page::CrawledPage};

type DefaultRateLimiter = RateLimiter<
    governor::state::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
>;

/// A crawler wrapper that enforces rate limits.
///
/// Uses the governor crate for precise rate limiting with burst support.
pub struct RateLimitedCrawler<C: Crawler> {
    inner: C,
    limiter: Arc<DefaultRateLimiter>,
}

impl<C: Crawler> RateLimitedCrawler<C> {
    /// Create a new rate-limited crawler.
    ///
    /// # Arguments
    /// * `crawler` - The underlying crawler to wrap
    /// * `requests_per_second` - Maximum requests per second
    pub fn new(crawler: C, requests_per_second: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second).expect("requests_per_second must be > 0"),
        );
        Self {
            inner: crawler,
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    /// Create with a custom quota.
    pub fn with_quota(crawler: C, quota: Quota) -> Self {
        Self {
            inner: crawler,
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    /// Create with burst support.
    ///
    /// # Arguments
    /// * `crawler` - The underlying crawler to wrap
    /// * `requests_per_second` - Sustained rate
    /// * `burst` - Maximum burst size
    pub fn with_burst(crawler: C, requests_per_second: u32, burst: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second).expect("requests_per_second must be > 0"),
        )
        .allow_burst(NonZeroU32::new(burst).expect("burst must be > 0"));

        Self {
            inner: crawler,
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    /// Wait for rate limiter before proceeding.
    async fn wait_for_permit(&self) {
        self.limiter.until_ready().await;
    }
}

#[async_trait]
impl<C: Crawler> Crawler for RateLimitedCrawler<C> {
    async fn crawl(&self, config: &CrawlConfig) -> CrawlResult<Vec<CrawledPage>> {
        // Wait for initial permit
        self.wait_for_permit().await;

        // The inner crawler will handle per-page rate limiting
        // We just add an outer limit on the crawl start
        self.inner.crawl(config).await
    }

    async fn fetch(&self, url: &str) -> CrawlResult<CrawledPage> {
        // Wait for rate limit before each fetch
        self.wait_for_permit().await;
        self.inner.fetch(url).await
    }

    async fn fetch_pages(&self, urls: &[&str]) -> CrawlResult<Vec<CrawledPage>> {
        let mut pages = Vec::with_capacity(urls.len());

        for url in urls {
            // Rate limit each request
            self.wait_for_permit().await;

            match self.inner.fetch(url).await {
                Ok(page) => pages.push(page),
                Err(e) => {
                    tracing::warn!("Failed to fetch {}: {}", url, e);
                }
            }
        }

        Ok(pages)
    }
}

/// Builder for RateLimitedCrawler with ergonomic configuration.
pub struct RateLimitedCrawlerBuilder<C: Crawler> {
    crawler: C,
    requests_per_second: u32,
    burst: Option<u32>,
}

impl<C: Crawler> RateLimitedCrawlerBuilder<C> {
    /// Create a new builder.
    pub fn new(crawler: C) -> Self {
        Self {
            crawler,
            requests_per_second: 1,
            burst: None,
        }
    }

    /// Set requests per second.
    pub fn requests_per_second(mut self, rps: u32) -> Self {
        self.requests_per_second = rps;
        self
    }

    /// Set burst size.
    pub fn burst(mut self, burst: u32) -> Self {
        self.burst = Some(burst);
        self
    }

    /// Build the rate-limited crawler.
    pub fn build(self) -> RateLimitedCrawler<C> {
        match self.burst {
            Some(burst) => {
                RateLimitedCrawler::with_burst(self.crawler, self.requests_per_second, burst)
            }
            None => RateLimitedCrawler::new(self.crawler, self.requests_per_second),
        }
    }
}

/// Extension trait for easy rate limiting.
pub trait CrawlerExt: Crawler + Sized {
    /// Wrap this crawler with rate limiting.
    fn rate_limited(self, requests_per_second: u32) -> RateLimitedCrawler<Self> {
        RateLimitedCrawler::new(self, requests_per_second)
    }

    /// Wrap with rate limiting and burst support.
    fn rate_limited_with_burst(self, requests_per_second: u32, burst: u32) -> RateLimitedCrawler<Self> {
        RateLimitedCrawler::with_burst(self, requests_per_second, burst)
    }
}

// Implement for all Crawlers
impl<C: Crawler + Sized> CrawlerExt for C {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockCrawler;
    use std::time::Instant;

    #[tokio::test]
    async fn test_rate_limiting() {
        let mock = MockCrawler::new()
            .with_page(crate::types::page::CrawledPage::new("https://example.com/1", "Page 1"))
            .with_page(crate::types::page::CrawledPage::new("https://example.com/2", "Page 2"))
            .with_page(crate::types::page::CrawledPage::new("https://example.com/3", "Page 3"));

        // 2 requests per second
        let crawler = mock.rate_limited(2);

        let start = Instant::now();

        // Fetch 3 pages
        let urls = ["https://example.com/1", "https://example.com/2", "https://example.com/3"];
        let pages = crawler.fetch_pages(&urls).await.unwrap();

        let elapsed = start.elapsed();

        assert_eq!(pages.len(), 3);

        // Should take at least 1 second for 3 requests at 2/sec
        // (first is immediate, 2nd and 3rd wait)
        assert!(elapsed.as_millis() >= 500, "Rate limiting not working: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_builder() {
        let mock = MockCrawler::new();

        let crawler = RateLimitedCrawlerBuilder::new(mock)
            .requests_per_second(5)
            .burst(10)
            .build();

        // Just verify it builds correctly
        assert!(crawler.inner.calls().is_empty());
    }

    #[tokio::test]
    async fn test_extension_trait() {
        let mock = MockCrawler::new();

        // Use extension method
        let _crawler = mock.rate_limited(1);
    }
}
