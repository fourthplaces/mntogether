//! Effects (side effects) for crawling domain
//!
//! Effects are thin orchestrators that watch events and execute work.
//!
//! - `crawler` - Mark no listings effect
//! - `pipeline` - Queued effects replacing the custom job system
//! - `discovery` - Page discovery via search

pub mod crawler;
pub mod discovery;
pub mod pipeline;

pub use crawler::mark_no_listings_effect;
pub use discovery::{discover_pages, DiscoveredPage};
pub use pipeline::crawling_pipeline_effect;
