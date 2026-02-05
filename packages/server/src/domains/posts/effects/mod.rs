//! Posts domain effects
//!
//! Effects watch FACT events and call cascade handlers.
//! Entry-point actions live in `actions/`, not here.

pub mod ai;
pub mod composite;
pub mod deduplication;
pub mod discovery;
pub mod post;
pub mod post_extraction;
pub mod post_operations;
pub mod post_report;
pub mod scraper;
pub mod syncing;
pub mod utils;

// llm_sync moved to actions/llm_sync.rs - it's business logic, not an effect

pub use composite::post_composite_effect;
// NOTE: Discovery queries have moved to domains/discovery/ (database-driven).
// The old hardcoded DISCOVERY_QUERIES in discovery.rs are kept for reference but no longer used.
pub use post::extract_domain;
pub use utils::*;

pub use crate::kernel::ServerDeps;
