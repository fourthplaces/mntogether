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
//! - WebsiteSnapshot model (individual page snapshots) [DEPRECATED]
//! - WebsiteAssessment model (AI-generated assessments)
//! - WebsiteResearch model (research data from Tavily searches)
//!
//! # Deprecation Note
//!
//! `WebsiteSnapshot` is deprecated. Use the extraction library's `extraction_pages` table
//! with `site_url` filtering instead. See `crawling/models/website_snapshot.rs` for details.

#![allow(deprecated)] // Re-exports deprecated WebsiteSnapshot

pub mod actions;
pub mod data;
pub mod effects;
pub mod events;
pub mod models;

// Explicit re-exports to avoid ambiguous glob re-exports
pub use data::{WebsiteAssessmentData, WebsiteData, WebsiteSearchResultData, WebsiteSnapshotData};
pub use effects::website_effect;
pub use events::WebsiteEvent;
pub use models::{
    CrawlStatus, TavilySearchQuery, TavilySearchResult, Website, WebsiteAssessment,
    WebsiteResearch, WebsiteResearchHomepage, WebsiteSearchResult, WebsiteSnapshot,
    WebsiteSnapshotId, WebsiteStatus,
};
