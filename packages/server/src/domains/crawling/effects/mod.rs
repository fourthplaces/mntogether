//! Effects (side effects) for crawling domain
//!
//! Effects are thin orchestrators that watch events and execute jobs.
//! The crawler effect handles the extraction → sync pipeline.

pub mod crawler;
pub mod discovery;
pub mod job_handlers;

pub use crawler::mark_no_listings_effect;
pub use discovery::{discover_pages, DiscoveredPage};
pub use job_handlers::register_crawling_jobs;
