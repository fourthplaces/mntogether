//! Effects (side effects) for crawling domain
//!
//! Effects are thin orchestrators that delegate to actions.
//! Handlers respond to internal cascade events in the multi-step workflow.

pub mod crawler;
pub mod discovery;
pub mod extraction;
pub mod handlers;

pub use crawler::*;
pub use discovery::{discover_pages, DiscoveredPage};
pub use handlers::{
    handle_extract_posts_from_pages, handle_mark_no_posts, handle_sync_crawled_posts,
};
