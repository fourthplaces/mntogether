//! Page summarization module (DEPRECATED)
//!
//! Generates AI summaries of pages for display in admin UI.
//! Post extraction now uses `posts::extraction::PostExtractor`.
//!
//! # Deprecation Notice
//!
//! This module is deprecated. Use the extraction library's summarization instead:
//! - `extraction::pipeline::ingest::ingest_with_ingestor()` - full ingestion with summarization
//! - `extraction::AI::summarize()` - direct summarization
//! - `extraction::CachedPage::hash_content()` - content hashing

#![allow(deprecated)]

pub mod summarize;
pub mod types;

pub use summarize::*;
pub use types::*;
