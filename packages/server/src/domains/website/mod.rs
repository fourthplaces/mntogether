//! Website domain - manages websites for scraping and approval workflows
//!
//! This domain contains:
//! - Website model (approval status, crawl configuration)
//! - WebsiteSnapshot model (individual page snapshots)
//! - WebsiteAssessment model (AI-generated assessments)
//! - WebsiteResearch model (research data from Tavily searches)

pub mod data;
pub mod edges;
pub mod models;

// Explicit re-exports to avoid ambiguous glob re-exports
pub use data::{WebsiteAssessmentData, WebsiteData, WebsiteSearchResultData, WebsiteSnapshotData};
pub use edges::{
    approve_website, crawl_website, query_pending_websites, query_website, query_websites,
    refresh_page_snapshot, reject_website, suspend_website,
};
pub use models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteSnapshot,
    WebsiteSnapshotId, WebsiteStatus,
};
