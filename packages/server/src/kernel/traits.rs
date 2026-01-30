// Trait definitions for dependency injection
//
// These are INFRASTRUCTURE traits only - no business logic.
// Business logic (like "extract needs") should be domain functions that use these traits.
//
// Naming convention: Base* for trait names (e.g., BaseWebScraper, BaseAI)

use anyhow::Result;
use async_trait::async_trait;

// =============================================================================
// Web Scraping Trait (Infrastructure)
// =============================================================================

/// Result of scraping a website
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub url: String,
    pub markdown: String,
    pub title: Option<String>,
}

#[async_trait]
pub trait BaseWebScraper: Send + Sync {
    /// Scrape a website and return clean text content
    async fn scrape(&self, url: &str) -> Result<ScrapeResult>;
}

// =============================================================================
// AI Trait (Infrastructure - Generic LLM capabilities)
// =============================================================================

#[async_trait]
pub trait BaseAI: Send + Sync {
    /// Complete a prompt with an LLM (returns raw text response)
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Complete a prompt expecting JSON response (returns raw JSON string)
    /// Parse with serde_json::from_str in calling code
    async fn complete_json(&self, prompt: &str) -> Result<String> {
        // Default implementation calls complete
        self.complete(prompt).await
    }
}

// =============================================================================
// Embedding Service Trait (Infrastructure)
// =============================================================================

#[async_trait]
pub trait BaseEmbeddingService: Send + Sync {
    /// Generate embedding for text (returns 1536-dimensional vector)
    async fn generate(&self, text: &str) -> Result<Vec<f32>>;
}

// =============================================================================
// Push Notification Trait (Infrastructure)
// =============================================================================

#[async_trait]
pub trait BasePushNotificationService: Send + Sync {
    /// Send a push notification to a push token
    async fn send_notification(
        &self,
        push_token: &str,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<()>;

    /// Send multiple notifications in batch
    async fn send_batch(
        &self,
        notifications: Vec<(&str, &str, &str, serde_json::Value)>,
    ) -> Result<()>;
}

// =============================================================================
// Twilio Service Trait (Infrastructure - SMS/OTP)
// =============================================================================

#[async_trait]
pub trait BaseTwilioService: Send + Sync {
    /// Send OTP code via SMS to phone number
    async fn send_otp(&self, phone_number: &str) -> Result<()>;

    /// Verify OTP code for phone number
    async fn verify_otp(&self, phone_number: &str, code: &str) -> Result<()>;
}

// =============================================================================
// PII Detection Trait (Infrastructure)
// =============================================================================

use crate::common::pii::{DetectionContext, PiiFindings, RedactionStrategy};

/// Result of PII detection and redaction
#[derive(Debug, Clone)]
pub struct PiiScrubResult {
    pub clean_text: String,
    pub findings: PiiFindings,
    pub pii_detected: bool,
}

#[async_trait]
pub trait BasePiiDetector: Send + Sync {
    /// Detect PII in text with context
    async fn detect(&self, text: &str, context: DetectionContext) -> Result<PiiFindings>;

    /// Detect and redact PII in one call (convenience method)
    async fn scrub(
        &self,
        text: &str,
        context: DetectionContext,
        strategy: RedactionStrategy,
    ) -> Result<PiiScrubResult>;
}

// =============================================================================
// Search Service Trait (Infrastructure)
// =============================================================================

use serde::{Deserialize, Serialize};

/// Result from a web search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f64,
    pub published_date: Option<String>,
}

/// Search service trait for discovering web content
#[async_trait]
pub trait BaseSearchService: Send + Sync {
    /// Search for content with optional filters
    async fn search(
        &self,
        query: &str,
        max_results: Option<usize>,
        search_depth: Option<&str>,
        days: Option<i32>,
    ) -> Result<Vec<SearchResult>>;
}
