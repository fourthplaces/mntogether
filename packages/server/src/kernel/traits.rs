// Trait definitions for dependency injection
//
// These are INFRASTRUCTURE traits only - no business logic.
// Business logic (like "extract needs") should be domain functions that use these traits.
//
// Naming convention: Base* for trait names (e.g., BaseEmbeddingService)
//
// NOTE: The BaseAI trait has been removed in favor of using `ai_client::OpenAi`
// directly. Use `extract<T>()` for structured JSON output or `.complete()` for
// simple text completions.

use anyhow::Result;
use async_trait::async_trait;

// =============================================================================
// Embedding Service Trait (Infrastructure)
// =============================================================================

#[async_trait]
pub trait BaseEmbeddingService: Send + Sync {
    /// Generate embedding for text (returns 1536-dimensional vector)
    async fn generate(&self, text: &str) -> Result<Vec<f32>>;
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

// =============================================================================
// Storage Service Trait (S3-compatible object storage)
// =============================================================================

#[async_trait]
pub trait BaseStorageService: Send + Sync {
    /// Generate a presigned PUT URL for direct browser upload.
    async fn presigned_upload_url(
        &self,
        key: &str,
        content_type: &str,
        expires_secs: u64,
    ) -> Result<String>;

    /// Delete an object by key.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Construct the public URL for a given key (no network call).
    fn public_url(&self, key: &str) -> String;
}

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
