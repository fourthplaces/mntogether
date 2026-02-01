//! Effects (side effects) for crawling domain
//!
//! Effects are thin orchestrators that delegate to domain functions.

pub mod crawler;
pub mod extraction;

pub use crawler::*;
pub use extraction::{hash_content, summarize_pages, synthesize_posts, PageToSummarize, SynthesisInput, SummarizedPage};
