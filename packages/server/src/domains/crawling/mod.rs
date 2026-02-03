//! Crawling domain - page discovery and content caching
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! This domain handles:
//! - Multi-page website crawling
//! - Page snapshot storage and deduplication
//! - AI-powered page summarization
//! - Post extraction from crawled content
//!
//! The crawling domain owns all page-level data (PageSnapshot, PageSummary, WebsiteSnapshot)
//! while the website domain owns the Website entity itself.

pub mod actions;
pub mod effects;
pub mod events;
pub mod models;

// Re-export events
pub use events::{CrawlEvent, CrawledPageInfo, PageExtractionResult};

// Re-export models
pub use models::{
    hash_to_hex, PageSnapshot, PageSnapshotId, PageSummary, PageSummaryId, WebsiteSnapshot,
    WebsiteSnapshotId,
};

// Re-export effects
pub use effects::{crawler_effect, hash_content, summarize_pages};
