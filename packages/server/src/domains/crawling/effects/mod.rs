//! Effects (side effects) for crawling domain
//!
//! Effects are thin orchestrators that watch events and execute work.
//!
//! - `pipeline` - All crawling effects (crawl, extract, investigate, join, sync, regenerate)
//! - `discovery` - Page discovery via search

pub mod discovery;
pub mod pipeline;

// Deprecated: crawler.rs merged into pipeline.rs
pub mod crawler;

pub use discovery::{discover_pages, DiscoveredPage};
pub use pipeline::handlers;
