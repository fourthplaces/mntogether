//! Crawling domain - system-level orchestration for website discovery and extraction
//!
//! # Architecture Overview (Restate Migration in Progress)
//!
//! The server has two related domains that work with the extraction library:
//!
//! | Domain | Purpose | Entry Point |
//! |--------|---------|-------------|
//! | **Extraction** | User-facing API | `extraction::actions::submit_url()` |
//! | **Crawling** | System orchestration | `crawling::workflows::CrawlWebsiteWorkflow` |
//!
//! **Extraction domain** handles explicit user requests (submit URL, run query).
//! **Crawling domain** handles system-level orchestration via durable workflows.
//!
//! Both use `ExtractionService` from the kernel as the underlying engine.
//!
//! # Components
//!
//! - `workflows/` - Durable workflows (CrawlWebsiteWorkflow)
//! - `activities/` - Business logic activities called by workflows (renamed from actions)
//! - `models/` - Data models (ExtractionPage)
//! - `effects/` - DEPRECATED: Being replaced by workflows
//! - `events/` - DEPRECATED: Being replaced by workflow state

pub mod activities;
pub mod effects; // TODO: Remove after migration
pub mod events;  // TODO: Remove after migration
pub mod models;
pub mod workflows;

// Re-export workflows
pub use workflows::*;

// Re-export events (temporary during migration)
pub use events::{CrawlEvent, CrawledPageInfo, PageExtractionResult};

// Re-export models
pub use models::ExtractionPage;

