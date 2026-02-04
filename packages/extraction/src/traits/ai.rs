//! AI trait for LLM operations.
//!
//! The AI trait abstracts LLM capabilities needed by the extraction pipeline:
//! - Summarization with recall signals
//! - Query expansion for better recall
//! - Partitioning candidates into buckets
//! - Evidence-grounded extraction
//! - Embedding generation

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{
    extraction::Extraction,
    page::CachedPage,
    summary::{Summary, SummaryResponse},
};

/// AI trait for LLM operations.
///
/// Implementations wrap specific LLM providers (OpenAI, Anthropic, etc.)
/// and handle the specifics of prompting and response parsing.
#[async_trait]
pub trait AI: Send + Sync {
    /// Summarize page content with recall-optimized signals.
    ///
    /// The summary should capture:
    /// - What the page offers (services, programs, opportunities)
    /// - What the page asks for (volunteers, donations, applications)
    /// - Calls to action (sign up, apply, contact, donate)
    /// - Key entities (organization names, locations, dates, contacts)
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse>;

    /// Expand query for recall (synonyms, related concepts).
    ///
    /// Returns a list of related search terms to improve recall.
    /// For example, "volunteer opportunities" might expand to:
    /// - "volunteering", "volunteer work", "community service"
    /// - "help wanted", "seeking volunteers"
    async fn expand_query(&self, query: &str) -> Result<Vec<String>>;

    /// Classify query intent for strategy selection.
    ///
    /// Returns the extraction strategy based on query analysis:
    /// - Collection: "Find all X" queries
    /// - Singular: Point lookups ("What is the phone number?")
    /// - Narrative: Aggregations ("Describe what this org does")
    async fn classify_query(&self, query: &str) -> Result<ExtractionStrategy>;

    /// Recall and partition in single call (simplified pipeline).
    ///
    /// Given a query and summaries, identifies distinct items and
    /// groups pages by which item they contribute to.
    async fn recall_and_partition(
        &self,
        query: &str,
        summaries: &[Summary],
    ) -> Result<Vec<Partition>>;

    /// Extract from page content with evidence grounding.
    ///
    /// Extracts information matching the query, citing sources for
    /// each claim. Returns gaps for missing information.
    async fn extract(
        &self,
        query: &str,
        pages: &[CachedPage],
        hints: Option<&[String]>,
    ) -> Result<Extraction>;

    /// Extract a single answer (for Singular strategy).
    ///
    /// Unlike `extract`, this returns a single piece of information
    /// rather than a list of items.
    async fn extract_single(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction> {
        // Default implementation uses regular extract
        self.extract(query, pages, None).await
    }

    /// Extract a narrative summary (for Narrative strategy).
    ///
    /// Aggregates information across pages into a cohesive description.
    async fn extract_narrative(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction> {
        // Default implementation uses regular extract with narrative hint
        self.extract(
            query,
            pages,
            Some(&["narrative".to_string(), "summary".to_string()]),
        )
        .await
    }

    /// Generate embedding for text.
    ///
    /// Returns a vector (typically 1024 or 1536 dimensions) for
    /// semantic similarity search.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch operation).
    ///
    /// More efficient than calling `embed` multiple times.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Default implementation calls embed sequentially
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}

/// Extraction strategy based on query intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionStrategy {
    /// "Find all X" - partition into buckets, extract each.
    ///
    /// Example: "Find volunteer opportunities"
    Collection,

    /// "Find specific info" - single answer from relevant pages.
    ///
    /// Example: "What is their phone number?"
    Singular,

    /// "Summarize/describe" - aggregate across all relevant pages.
    ///
    /// Example: "What does this organization do?"
    Narrative,
}

impl Default for ExtractionStrategy {
    fn default() -> Self {
        Self::Collection
    }
}

/// A partition of pages grouped by a distinct item.
#[derive(Debug, Clone)]
pub struct Partition {
    /// Brief title for this item
    pub title: String,

    /// URLs of pages that contribute to this item
    pub urls: Vec<String>,

    /// Why these pages were grouped together
    pub rationale: String,
}

impl Partition {
    /// Create a new partition.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            urls: Vec::new(),
            rationale: String::new(),
        }
    }

    /// Add a URL to this partition.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.urls.push(url.into());
        self
    }

    /// Add multiple URLs to this partition.
    pub fn with_urls(mut self, urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.urls.extend(urls.into_iter().map(|u| u.into()));
        self
    }

    /// Set the rationale.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = rationale.into();
        self
    }
}

/// Response from query classification.
#[derive(Debug, Clone)]
pub struct ClassificationResponse {
    /// The determined strategy
    pub strategy: ExtractionStrategy,

    /// Confidence in the classification (0.0 to 1.0)
    pub confidence: f32,

    /// Reasoning for the classification
    pub reasoning: String,
}
