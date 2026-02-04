//! Posts domain actions - entry-point business logic
//!
//! Called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return final models/results.

pub mod core;
pub mod deduplication;
pub mod llm_sync;
pub mod reports;
pub mod scraping;
pub mod tags;
// transformer module deprecated - use ai.generate_structured() directly
// See crawling/effects/handlers.rs for example usage

// Re-export for convenience
pub use core::*;
pub use deduplication::{deduplicate_posts, DeduplicationResult};
pub use llm_sync::{llm_sync_posts, LlmSyncResult};
pub use reports::*;
pub use scraping::{submit_resource_link, SubmitResourceLinkResult};
pub use tags::{add_post_tag, remove_post_tag, update_post_tags, TagInput};
