//! Ingestor implementations for various content sources.
//!
//! This module provides concrete implementations of the `Ingestor` trait
//! for different content sources.
//!
//! # Available Ingestors
//!
//! - `HttpIngestor` - Basic HTTP crawling with rate limiting
//! - `FirecrawlIngestor` - Firecrawl API (requires `firecrawl` feature)
//! - `MockIngestor` - For testing
//!
//! # Example
//!
//! ```rust,ignore
//! use extraction::ingestors::{HttpIngestor, ValidatedIngestor};
//! use extraction::traits::ingestor::DiscoverConfig;
//!
//! let ingestor = ValidatedIngestor::new(HttpIngestor::new());
//! let config = DiscoverConfig::new("https://example.com").with_limit(10);
//! let pages = ingestor.discover(&config).await?;
//! ```

mod http;
mod mock;

#[cfg(feature = "firecrawl")]
mod firecrawl;

pub use http::HttpIngestor;
pub use mock::{MockIngestor, MockIngestorBuilder};

#[cfg(feature = "firecrawl")]
pub use firecrawl::FirecrawlIngestor;

// Re-export from traits for convenience
pub use crate::traits::ingestor::{DiscoverConfig, Ingestor, RawPage, ValidatedIngestor};
