//! Crawling domain - system-level orchestration for website discovery and extraction
//!
//! # Architecture Overview (Restate Migration in Progress)
//!
//! The server has two related domains that work with the extraction library:
//!
//! | Domain | Purpose | Entry Point |
//! |--------|---------|-------------|
//! | **Extraction** | User-facing API | `extraction::activities::submit_url()` |
//! | **Crawling** | System orchestration | `crawling::restate::CrawlWebsiteWorkflow` |
//!
//! **Extraction domain** handles explicit user requests (submit URL, run query).
//! **Crawling domain** handles system-level orchestration via durable workflows.
//!
//! Both use `ExtractionService` from the kernel as the underlying engine.
//!
//! # Components
//!
//! - `restate/` - Durable workflows (CrawlWebsiteWorkflow)
//! - `activities/` - Business logic activities called by workflows (renamed from actions)
//! - `models/` - Data models (ExtractionPage)
//! - `effects/` - DEPRECATED: Being replaced by restate workflows
//! - `events/` - DEPRECATED: Being replaced by restate workflow state

pub mod activities;
pub mod models;
pub mod types;
pub mod restate;

// Re-export restate types
pub use restate::*;

// Re-export types
pub use types::{WebsiteIngested, NarrativesExtracted, PostsSynced};

// Re-export models
pub use models::ExtractionPage;

