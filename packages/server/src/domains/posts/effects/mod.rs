//! Posts domain effects
//!
//! Effects watch FACT events and call cascade handlers.
//! Entry-point actions live in `actions/`, not here.

pub mod agentic_extraction;
pub mod ai;
pub mod composite;
pub mod deduplication;
pub mod discovery;
pub mod extraction_tools;
pub mod llm_sync;
pub mod post;
pub mod post_extraction;
pub mod post_operations;
pub mod post_report;
pub mod scraper;
pub mod sync;
pub mod syncing;
pub mod utils;

pub use composite::post_composite_effect;
pub use discovery::{run_discovery_searches, DiscoveryResult, DEFAULT_LOCATION, DISCOVERY_QUERIES};
pub use post::extract_domain;
pub use utils::*;

pub use crate::kernel::ServerDeps;
