//! Summary types - recall-optimized page summaries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A recall-optimized summary of a page.
///
/// Summaries are not just readable text - they're designed to maximize
/// recall during search. They preserve calls-to-action, offers, asks,
/// and key entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// URL of the summarized page
    pub url: String,

    /// Site URL this page belongs to
    pub site_url: String,

    /// The summary text (2-3 sentences)
    pub text: String,

    /// Structured signals extracted for recall
    pub signals: RecallSignals,

    /// Detected language of the content
    pub language: Option<String>,

    /// When this summary was created
    pub created_at: DateTime<Utc>,

    /// Hash of the summarization prompt.
    ///
    /// Used for cache invalidation - if the prompt changes, summaries
    /// should be regenerated.
    pub prompt_hash: String,

    /// Hash of the source content.
    ///
    /// Used to detect when the source page has changed.
    pub content_hash: String,

    /// Pre-computed embedding for semantic search (optional).
    ///
    /// May be stored separately for efficiency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl Summary {
    /// Create a new summary.
    pub fn new(
        url: impl Into<String>,
        site_url: impl Into<String>,
        text: impl Into<String>,
        content_hash: impl Into<String>,
        prompt_hash: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            site_url: site_url.into(),
            text: text.into(),
            signals: RecallSignals::default(),
            language: None,
            created_at: Utc::now(),
            prompt_hash: prompt_hash.into(),
            content_hash: content_hash.into(),
            embedding: None,
        }
    }

    /// Hash a prompt for cache invalidation.
    pub fn hash_prompt(prompt: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Set the recall signals.
    pub fn with_signals(mut self, signals: RecallSignals) -> Self {
        self.signals = signals;
        self
    }

    /// Set the language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set the embedding.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Check if this summary was generated with a different prompt.
    pub fn is_prompt_stale(&self, current_prompt_hash: &str) -> bool {
        self.prompt_hash != current_prompt_hash
    }

    /// Check if the source content has changed.
    pub fn is_content_stale(&self, current_content_hash: &str) -> bool {
        self.content_hash != current_content_hash
    }

    /// Get combined text for embedding (summary + signals).
    pub fn embedding_text(&self) -> String {
        let mut parts = vec![self.text.clone()];

        if !self.signals.calls_to_action.is_empty() {
            parts.push(format!("CTAs: {}", self.signals.calls_to_action.join(", ")));
        }
        if !self.signals.offers.is_empty() {
            parts.push(format!("Offers: {}", self.signals.offers.join(", ")));
        }
        if !self.signals.asks.is_empty() {
            parts.push(format!("Asks: {}", self.signals.asks.join(", ")));
        }
        if !self.signals.entities.is_empty() {
            parts.push(format!("Entities: {}", self.signals.entities.join(", ")));
        }

        parts.join("\n")
    }
}

/// Structured signals extracted from a page for recall optimization.
///
/// These signals help find pages that might not match keyword searches
/// but are semantically relevant.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecallSignals {
    /// Calls to action (e.g., "sign up", "apply now", "donate")
    #[serde(default)]
    pub calls_to_action: Vec<String>,

    /// Things the page offers (services, programs, opportunities)
    #[serde(default)]
    pub offers: Vec<String>,

    /// Things the page asks for (volunteers, donations, applications)
    #[serde(default)]
    pub asks: Vec<String>,

    /// Key entities (organization names, locations, dates, contacts)
    #[serde(default)]
    pub entities: Vec<String>,
}

impl RecallSignals {
    /// Create new recall signals.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a call to action.
    pub fn with_cta(mut self, cta: impl Into<String>) -> Self {
        self.calls_to_action.push(cta.into());
        self
    }

    /// Add an offer.
    pub fn with_offer(mut self, offer: impl Into<String>) -> Self {
        self.offers.push(offer.into());
        self
    }

    /// Add an ask.
    pub fn with_ask(mut self, ask: impl Into<String>) -> Self {
        self.asks.push(ask.into());
        self
    }

    /// Add an entity.
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entities.push(entity.into());
        self
    }

    /// Check if there are any signals.
    pub fn is_empty(&self) -> bool {
        self.calls_to_action.is_empty()
            && self.offers.is_empty()
            && self.asks.is_empty()
            && self.entities.is_empty()
    }

    /// Get total count of all signals.
    pub fn count(&self) -> usize {
        self.calls_to_action.len()
            + self.offers.len()
            + self.asks.len()
            + self.entities.len()
    }
}

/// Response from the AI summarization endpoint.
///
/// This is what the AI returns, which gets converted to a Summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResponse {
    /// The summary text
    pub summary: String,

    /// Structured signals
    pub signals: RecallSignals,

    /// Detected language
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_hash() {
        let hash1 = Summary::hash_prompt("Summarize this page");
        let hash2 = Summary::hash_prompt("Summarize this page");
        let hash3 = Summary::hash_prompt("Summarize this page differently");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_is_prompt_stale() {
        let summary = Summary::new(
            "https://example.com",
            "https://example.com",
            "Summary text",
            "content_hash",
            Summary::hash_prompt("old prompt"),
        );

        assert!(!summary.is_prompt_stale(&Summary::hash_prompt("old prompt")));
        assert!(summary.is_prompt_stale(&Summary::hash_prompt("new prompt")));
    }

    #[test]
    fn test_embedding_text() {
        let summary = Summary::new(
            "https://example.com",
            "https://example.com",
            "Main summary text",
            "hash",
            "prompt_hash",
        )
        .with_signals(
            RecallSignals::new()
                .with_cta("Sign up now")
                .with_offer("Free consultation"),
        );

        let text = summary.embedding_text();
        assert!(text.contains("Main summary text"));
        assert!(text.contains("Sign up now"));
        assert!(text.contains("Free consultation"));
    }
}
