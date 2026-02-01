//! Crawling domain - page discovery and content caching
//!
//! This domain handles:
//! - Multi-page website crawling
//! - Page snapshot storage and deduplication
//! - AI-powered page summarization
//! - Post extraction from crawled content
//!
//! The crawling domain owns all page-level data (PageSnapshot, PageSummary, WebsiteSnapshot)
//! while the website domain owns the Website entity itself.

pub mod commands;
pub mod effects;
pub mod events;
pub mod machines;
pub mod models;

// Re-export commands
pub use commands::CrawlCommand;

// Re-export events
pub use events::{CrawlEvent, CrawledPageInfo, PageExtractionResult};

// Re-export models
pub use models::{
    PageSnapshot, PageSnapshotId, PageSummary, PageSummaryId, WebsiteSnapshot, WebsiteSnapshotId,
    hash_to_hex,
};

// Re-export effects
pub use effects::{CrawlerEffect, hash_content, summarize_pages, synthesize_posts};

// Re-export machines
pub use machines::CrawlMachine;
