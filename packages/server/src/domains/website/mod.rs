//! Website domain - manages websites for scraping and approval workflows

pub mod activities;
pub mod data;
pub mod models;
pub mod workflows;

// Explicit re-exports to avoid ambiguous glob re-exports
pub use data::{WebsiteAssessmentData, WebsiteData, WebsiteSearchResultData};
pub use models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteStatus,
};
