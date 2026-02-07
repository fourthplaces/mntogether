//! Website domain - manages websites for scraping and approval workflows
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Components:
//! - actions: Entry-point business logic called directly from GraphQL via process()
//! - effects: Event handlers that respond to fact events
//!
//! Models:
//! - Website model (approval status, crawl configuration)
//! - WebsiteAssessment model (AI-generated assessments)
//! - WebsiteResearch model (research data from Tavily searches)

pub mod activities;
pub mod data;
pub mod effects;
pub mod events;
pub mod models;

// Explicit re-exports to avoid ambiguous glob re-exports
pub use data::{WebsiteAssessmentData, WebsiteData, WebsiteSearchResultData};
pub use events::WebsiteEvent;
pub use events::approval::WebsiteApprovalEvent;
pub use models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteStatus,
};
