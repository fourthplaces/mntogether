pub mod page_snapshot;
pub mod page_summary;

pub use page_snapshot::*;
pub use page_summary::*;

// Re-export website models from the website domain for backward compatibility
pub use crate::domains::website::models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteSnapshot,
    WebsiteSnapshotId, WebsiteStatus,
};
