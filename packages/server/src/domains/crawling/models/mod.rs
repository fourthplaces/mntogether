//! Crawling domain models
//!
//! Models for page discovery, snapshot storage, and content caching.
//!
//! # Deprecation Notice
//!
//! Many models in this module are deprecated in favor of the extraction library:
//! - `PageSnapshot` → `extraction::CachedPage`
//! - `PageSummary` → `extraction::Summary`
//! - `WebsiteSnapshot` → Junction table, no replacement needed (extraction_pages has site_url)
//!
//! Use `ExtractionService` and the extraction library's types instead.

#![allow(deprecated)] // Re-exports deprecated items for backward compatibility

pub mod page_extraction;
pub mod page_snapshot;
pub mod page_summary;
pub mod website_snapshot;

pub use page_extraction::*;
#[allow(deprecated)]
pub use page_snapshot::*;
pub use page_summary::*;
#[allow(deprecated)]
pub use website_snapshot::*;
