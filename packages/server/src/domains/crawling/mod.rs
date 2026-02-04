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
//! # Domain Ownership
//!
//! This domain is being migrated to use the extraction library. Current ownership:
//!
//! | Component | Owner | Status |
//! |-----------|-------|--------|
//! | `ingest_website()` | Crawling | **NEW** - uses extraction library |
//! | `crawl_website()` | Crawling | DEPRECATED - uses old BaseWebScraper |
//! | `PageSnapshot` | Crawling | DEPRECATED - use extraction_pages |
//! | `PageSummary` | Crawling | DEPRECATED - use extraction_summaries |
//! | `WebsiteSnapshot` | Crawling | DEPRECATED - use site_url on extraction_pages |
//! | Event cascade | Crawling | KEEP - orchestrates post extraction |
//!
//! # Migration Path
//!
//! **Old flow (deprecated):**
//! ```text
//! crawl_website() → BaseWebScraper → page_snapshots → WebsiteCrawled event
//! ```
//!
//! **New flow (preferred):**
//! ```text
//! ingest_website() → ExtractionService → extraction_pages → WebsiteIngested event
//! ```
//!
//! # Why Two Domains?
//!
//! - **Extraction domain** is simple: GraphQL → action → ExtractionService → result
//! - **Crawling domain** is complex: event-driven orchestration, background jobs, cascade
//!
//! Keeping them separate maintains single responsibility principle.
//!
//! # Components
//!
//! - `actions/` - Business logic (ingest_website, crawl_website deprecated)
//! - `effects/` - Event handlers for the crawl cascade
//! - `models/` - Data models (PageSnapshot deprecated, PageSummary deprecated)
//! - `events/` - Crawl events (WebsiteIngested, WebsiteCrawled deprecated)

#![allow(deprecated)] // Re-exports deprecated items for backward compatibility

pub mod actions;
pub mod effects;
pub mod events;
pub mod ingestors;
pub mod jobs;
pub mod models;

// Re-export events
pub use events::{CrawlEvent, CrawledPageInfo, PageExtractionResult};

// Re-export models
pub use models::{
    hash_to_hex, PageSnapshot, PageSnapshotId, PageSummary, PageSummaryId, WebsiteSnapshot,
    WebsiteSnapshotId,
};

// Re-export effects
pub use effects::{crawler_effect, hash_content, summarize_pages};

// Re-export ingestors
pub use ingestors::PageSnapshotIngestor;

// Re-export jobs
pub use jobs::{
    execute_crawl_website_job, execute_regenerate_posts_job, CrawlWebsiteJob, JobExecutionResult,
    JobInfo, RegeneratePostsJob,
};
