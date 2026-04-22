// Trait definitions for dependency injection
//
// These are INFRASTRUCTURE traits only - no business logic.
// Naming convention: Base* for trait names (e.g., BaseTwilioService)

use anyhow::Result;
use async_trait::async_trait;

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

    /// Server-side upload. Used by the Root Signal media ingest pipeline:
    /// the server fetches + normalises an external image, then writes the
    /// WebP bytes here. Editor uploads still use the presigned path.
    async fn put_object(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<()>;

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
