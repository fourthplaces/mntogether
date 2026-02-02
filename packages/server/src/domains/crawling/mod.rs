//! Crawling domain - page discovery and content caching
//!
//! Architecture (seesaw 0.3.0):
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...
//!
//! Components:
//! - events: Request events (user intent) and fact events (what happened)
//! - effects: Thin dispatcher that routes request events to handlers
//! - edges/internal: React to fact events, emit new request events
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
pub mod edges;
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
pub use effects::{hash_content, summarize_pages, synthesize_posts, CrawlerEffect};
