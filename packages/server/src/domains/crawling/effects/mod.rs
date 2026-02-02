//! Effects (side effects) for crawling domain
//!
//! Effects are thin orchestrators that delegate to actions.
//! Handlers respond to internal cascade events in the multi-step workflow.

pub mod crawler;
pub mod extraction;
pub mod handlers;

pub use crawler::*;
pub use extraction::{hash_content, summarize_pages, synthesize_posts, PageToSummarize, SynthesisInput, SummarizedPage};
pub use handlers::{
    handle_extract_from_pages, handle_mark_no_posts, handle_retry_crawl, handle_sync_crawled_posts,
};
