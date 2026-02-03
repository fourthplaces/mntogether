//! Crawler implementations.
//!
//! Provides various crawling strategies:
//! - `HttpCrawler` - Direct HTTP crawling with link following
//! - `RateLimitedCrawler` - Wrapper that adds rate limiting
//! - `ValidatedCrawler` - Wrapper that validates URLs for security
//! - `InformedCrawler` - Query-driven discovery combining HTTP + search
//! - `TavilyCrawler` - Search-based page discovery using Tavily API
//! - `RobotsTxt` - robots.txt parsing for respectful crawling

pub mod http;
pub mod informed;
pub mod rate_limited;
pub mod robots;
pub mod tavily;

pub use http::HttpCrawler;
pub use informed::{InformedCrawler, MockSearchService, SearchService, TavilySearchService};
pub use rate_limited::RateLimitedCrawler;
pub use robots::{fetch_robots_txt, RobotsTxt};
pub use tavily::{QueryGenerator, TavilyCrawler};

// Re-export the validated crawler from traits
pub use crate::traits::crawler::{UrlValidator, ValidatedCrawler};
