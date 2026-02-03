//! Crawling domain models
//!
//! Models for page discovery, snapshot storage, and content caching.

pub mod page_extraction;
pub mod page_snapshot;
pub mod page_summary;
pub mod website_snapshot;

pub use page_extraction::*;
pub use page_snapshot::*;
pub use page_summary::*;
pub use website_snapshot::*;
