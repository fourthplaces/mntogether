//! Website domain - manages websites for scraping and approval workflows
//!
//! Architecture (seesaw 0.3.0):
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...
//!
//! Components:
//! - events: Request events (user intent) and fact events (what happened)
//! - effects: Thin dispatcher that routes request events to handlers
//! - edges/internal: React to fact events, emit new request events (currently empty)
//! - edges/mutation: GraphQL mutations that emit request events
//! - edges/query: GraphQL queries (read-only)
//!
//! Models:
//! - Website model (approval status, crawl configuration)
//! - WebsiteSnapshot model (individual page snapshots)
//! - WebsiteAssessment model (AI-generated assessments)
//! - WebsiteResearch model (research data from Tavily searches)

pub mod actions;
pub mod data;
pub mod edges;
pub mod effects;
pub mod events;
pub mod models;

// Explicit re-exports to avoid ambiguous glob re-exports
pub use data::{WebsiteAssessmentData, WebsiteData, WebsiteSearchResultData, WebsiteSnapshotData};
pub use edges::{
    approve_website, crawl_website, query_pending_websites, query_website, query_websites,
    refresh_page_snapshot, reject_website, suspend_website,
};
pub use effects::WebsiteEffect;
pub use events::WebsiteEvent;
pub use models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteSnapshot,
    WebsiteSnapshotId, WebsiteStatus,
};
