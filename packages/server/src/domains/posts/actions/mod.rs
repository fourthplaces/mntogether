//! Posts domain actions - entry-point business logic
//!
//! Called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return final models/results.

pub mod agentic_extraction;
pub mod core;
pub mod deduplication;
pub mod llm_sync;
pub mod reports;
pub mod scraping;
pub mod tags;

// Re-export for convenience
pub use core::*;
pub use deduplication::{deduplicate_posts, DeduplicationResult};
pub use llm_sync::{llm_sync_posts, SyncResult};
pub use reports::*;
pub use scraping::{
    refresh_page_snapshot, scrape_source, submit_resource_link, RefreshPageSnapshotResult,
    ScrapeJobResult, SubmitResourceLinkResult,
};
pub use tags::{add_post_tag, remove_post_tag, update_post_tags, TagInput};
