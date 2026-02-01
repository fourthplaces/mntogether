//! DEPRECATED: Use `crate::domains::crawling::models` instead.
//!
//! This module re-exports from crawling domain for backward compatibility.

// Re-export from crawling domain (the new home for these models)
pub use crate::domains::crawling::models::{
    PageSnapshot, PageSnapshotId, PageSummary, PageSummaryId, WebsiteSnapshot, WebsiteSnapshotId,
    hash_to_hex,
};

// Re-export website models from the website domain for backward compatibility
pub use crate::domains::website::models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteStatus,
};
