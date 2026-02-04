// Trait definitions for dependency injection
//
// These are INFRASTRUCTURE traits only - no business logic.
// Business logic (like "extract needs") should be domain functions that use these traits.
//
// Naming convention: Base* for trait names (e.g., BaseAI, BaseEmbeddingService)

use anyhow::Result;
use async_trait::async_trait;

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

    /// Complete a prompt with a specific model (returns raw text response)
    /// If model is None, uses the default model
    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        // Default implementation ignores model and calls complete
        let _ = model;
        self.complete(prompt).await
    }

    /// Complete a prompt expecting JSON response with a specific model
    async fn complete_json_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        // Default implementation ignores model and calls complete_json
        let _ = model;
        self.complete_json(prompt).await
    }

    /// Generate structured output with a JSON schema
    /// Returns JSON string conforming to the provided schema
    async fn generate_structured(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value,
    ) -> Result<String> {
        // Default implementation ignores schema and just prompts for JSON
        let _ = schema;
        let combined = format!(
            "{}\n\nRespond with valid JSON.\n\n{}",
            system_prompt, user_prompt
        );
        self.complete_json(&combined).await
    }

    /// Generate with tool calling support
    /// Returns the assistant's response which may include tool_calls
    async fn generate_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Default implementation - not all providers support tools
        let _ = tools;
        // Just return the last user message content as if processed
        let last_user = messages
            .iter()
            .rev()
            .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let response = self.complete(last_user).await?;
        Ok(serde_json::json!({"content": response}))
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
