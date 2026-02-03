//! Extraction domain events
//!
//! Events emitted by extraction actions for observability and side effects.

use serde::{Deserialize, Serialize};

/// Events emitted by extraction operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionEvent {
    /// A URL was submitted for extraction
    UrlSubmitted {
        url: String,
        query: Option<String>,
        extractions_count: usize,
    },

    /// An extraction query was triggered
    ExtractionTriggered {
        query: String,
        site: Option<String>,
        extractions_count: usize,
    },

    /// A site was ingested
    SiteIngested {
        site_url: String,
        pages_crawled: usize,
        pages_summarized: usize,
    },

    /// Extraction failed
    ExtractionFailed {
        query: String,
        error: String,
    },
}
