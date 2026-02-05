//! Crawling domain - system-level orchestration for website discovery and extraction
//!
//! # Architecture Overview
//!
//! The server has two related domains that work with the extraction library:
//!
//! | Domain | Purpose | Entry Point |
//! |--------|---------|-------------|
//! | **Extraction** | User-facing API | `extraction::actions::submit_url()` |
//! | **Crawling** | System orchestration | `crawling::actions::ingest_website()` |
//!
//! **Extraction domain** handles explicit user requests (submit URL, run query).
//! **Crawling domain** handles system-level orchestration (background crawls, event cascade).
//!
//! Both use `ExtractionService` from the kernel as the underlying engine.
//!
//! # Components
//!
//! - `actions/` - Business logic (ingest_website)
//! - `effects/` - Event handlers for the crawl cascade
//! - `models/` - Data models (ExtractionPage)
//! - `events/` - Crawl events (WebsiteIngested)

pub mod actions;
pub mod effects;
pub mod events;
pub mod jobs;
pub mod models;

// Re-export events
pub use events::{CrawlEvent, CrawledPageInfo, PageExtractionResult};

// Re-export models
pub use models::ExtractionPage;

// Re-export effects (job chaining now handled by JobRunner + job_handlers)
pub use effects::mark_no_listings_effect;

// Re-export job types and helpers
pub use jobs::{
    CrawlWebsiteJob, ExtractPostsJob, JobInfo, RegeneratePostsJob, RegenerateSinglePostJob,
    SyncPostsJob,
};
