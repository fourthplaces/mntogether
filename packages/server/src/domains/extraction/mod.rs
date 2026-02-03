//! Extraction domain - user-facing interface for query-driven information extraction
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
//! This domain handles **explicit user requests** (submit URL, run query).
//! The crawling domain handles **system-level orchestration** (background crawls, event cascade).
//!
//! # Domain Ownership
//!
//! | Component | Owner | Purpose |
//! |-----------|-------|---------|
//! | `submit_url()` | Extraction | User submits a URL for extraction |
//! | `trigger_extraction()` | Extraction | User runs extraction query on site |
//! | `ingest_site()` | Extraction | Admin ingests entire site |
//! | GraphQL types | Extraction | User-facing API data types |
//!
//! # Architecture
//!
//! ```text
//! GraphQL → actions → ExtractionService → extraction library → results
//! ```
//!
//! This is a **simple, direct path** - no event cascade, no background jobs.
//! For system-level orchestration, see the crawling domain.
//!
//! # Components
//!
//! - `actions`: Business logic for URL submission and extraction queries
//! - `data`: GraphQL data types (Extraction, Gap, Source, etc.)
//! - `events`: Events for observability and audit logging
//!
//! # See Also
//!
//! - `domains::crawling` - System-level orchestration (background crawls, event cascade)
//! - `kernel::extraction_service` - The underlying ExtractionService wrapper

pub mod actions;
pub mod data;
pub mod events;

// Re-exports
pub use actions::{ingest_site, submit_url, submit_url_one, trigger_extraction, trigger_extraction_one, IngestSiteResult};
pub use data::{
    ConflictData, ConflictingClaimData, ExtractionData, ExtractionStatusData, GapData,
    GroundingGradeData, SourceData, SourceRoleData, SubmitUrlInput, SubmitUrlResult,
    TriggerExtractionInput, TriggerExtractionResult,
};
pub use events::ExtractionEvent;
