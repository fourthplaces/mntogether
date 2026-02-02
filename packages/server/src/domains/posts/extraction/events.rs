//! Post extraction events
//!
//! Events for the AI extraction workflow.

use crate::common::{JobId, WebsiteId};

/// Post extraction domain events
#[derive(Debug, Clone)]
pub enum PostExtractionEvent {
    // =========================================================================
    // Fact Events (from effects - what actually happened)
    // =========================================================================

    /// Posts were extracted from crawled pages and synced to database
    PostsExtractedAndSynced {
        website_id: WebsiteId,
        job_id: JobId,
        pages_processed: usize,
        posts_extracted: usize,
        posts_created: usize,
        posts_updated: usize,
    },

    /// Post extraction failed
    ExtractionFailed {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    /// No posts were found in any of the crawled pages
    NoPostsFound {
        website_id: WebsiteId,
        job_id: JobId,
        pages_processed: usize,
    },
}
