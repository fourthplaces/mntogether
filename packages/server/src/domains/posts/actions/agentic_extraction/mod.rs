//! Agentic extraction actions (placeholder for future split)
//!
//! This module will eventually contain the split agentic extraction logic from
//! `posts/effects/agentic_extraction.rs`. The current file structure plan:
//!
//! ```text
//! posts/actions/agentic_extraction/
//! ├── mod.rs           # This file - module exports
//! ├── types.rs         # Data structures (PostCandidate, EnrichedPost, etc.)
//! ├── tools.rs         # Tool definitions for the agent loop
//! ├── extraction.rs    # Candidate extraction and enrichment
//! ├── tool_loop.rs     # Tool execution loop
//! ├── merging.rs       # Post merging and deduplication
//! ├── pipeline.rs      # Main extraction pipelines
//! ├── storage.rs       # Storage and sync functions
//! └── conversions.rs   # Type conversions
//! ```
//!
//! For now, use `posts::effects::agentic_extraction` directly.
//! This module will be populated as the refactoring progresses.

// Re-exports from the current location (for gradual migration)
pub use crate::domains::posts::effects::agentic_extraction::{
    extract_from_page, extract_from_website, to_extracted_posts, EnrichedPost, PostCandidate,
    WebsiteExtractionResult,
};
