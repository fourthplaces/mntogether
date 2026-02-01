//! Types for two-pass post extraction
//!
//! Pass 1: Summarize each page -> PageSummary { url, content }
//! Pass 2: Synthesize all summaries -> Posts with Tags

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Input for Pass 1: a page to summarize
#[derive(Debug, Clone)]
pub struct PageToSummarize {
    pub snapshot_id: Uuid,
    pub url: String,
    pub raw_content: String,   // Raw HTML/markdown from snapshot
    pub content_hash: String,  // For cache lookup
}

/// Output from Pass 1: meaningful content extracted from a page
#[derive(Debug, Clone)]
pub struct SummarizedPage {
    pub snapshot_id: Uuid,
    pub url: String,
    pub content: String, // Extracted meaningful content
}

/// Input for Pass 2: all summaries for a website
#[derive(Debug, Clone)]
pub struct SynthesisInput {
    pub website_domain: String,
    pub pages: Vec<SummarizedPage>,
}

/// Output from Pass 2: extracted post with tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPost {
    pub title: String,
    pub tldr: String,
    pub description: String,
    /// Primary audience for this post - who the post is FOR
    /// Valid values: "recipient", "volunteer", "donor", "job-seeker", "participant"
    #[serde(default)]
    pub primary_audience: Option<String>,
    #[serde(default)]
    pub contact: Option<ExtractedContact>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub tags: Vec<ExtractedTag>,
    #[serde(default)]
    pub source_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContact {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTag {
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

impl ExtractedTag {
    pub fn new(kind: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            value: value.into(),
            display_name: None,
        }
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
}
