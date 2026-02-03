//! Page summarization module
//!
//! Generates AI summaries of pages for display in admin UI.
//! Post extraction now uses agentic extraction (see posts/effects/agentic_extraction.rs)

pub mod summarize;
pub mod types;

pub use summarize::*;
pub use types::*;
