//! AI implementation using OpenAI
//!
//! This module provides `OpenAIClient`, which wraps the extraction library's
//! OpenAI implementation to provide backwards compatibility with existing
//! server code.
//!
//! The actual AI implementation lives in `extraction::ai::OpenAI`. This module
//! provides the bridge between that and the server's `BaseAI` trait.

use anyhow::{Context, Result};
use async_trait::async_trait;
use extraction::ai::OpenAI as ExtractionOpenAI;
use extraction::AI;

use super::{BaseAI, BaseEmbeddingService};

/// OpenAI implementation of AI capabilities.
///
/// This wraps the extraction library's `OpenAI` implementation to provide
/// backwards compatibility with existing server code that uses `OpenAIClient`.
///
/// # Migration Note
///
/// New code should use `ExtractionAIBridge` directly from `extraction_bridge`,
/// or the extraction library's `OpenAI` implementation.
#[derive(Clone)]
pub struct OpenAIClient {
    inner: ExtractionOpenAI,
}

impl OpenAIClient {
    /// Create a new OpenAI client with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            inner: ExtractionOpenAI::new(api_key),
        }
    }

    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let inner = ExtractionOpenAI::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
        Ok(Self { inner })
    }

    /// Get the underlying extraction OpenAI instance.
    pub fn inner(&self) -> &ExtractionOpenAI {
        &self.inner
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        self.inner.api_key()
    }
}

#[async_trait]
impl BaseAI for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.inner
            .complete(prompt)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        self.complete(prompt).await
    }

    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        self.inner
            .complete_with_model(prompt, model)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn complete_json_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        self.complete_with_model(prompt, model).await
    }

    async fn generate_structured(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value,
    ) -> Result<String> {
        self.inner
            .generate_structured(system_prompt, user_prompt, schema)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn generate_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.inner
            .generate_with_tools(messages, tools)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[async_trait]
impl BaseEmbeddingService for OpenAIClient {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        self.inner
            .embed(text)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to generate embedding")
    }
}

impl OpenAIClient {
    /// Create an embedding for the given text.
    ///
    /// Alias for `BaseEmbeddingService::generate` for code that expects
    /// `create_embedding` method name.
    pub async fn create_embedding(&self, text: &str) -> Result<Vec<f32>> {
        self.inner
            .embed(text)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to create embedding")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_compiles() {
        fn _assert_base_ai<T: BaseAI>() {}
        fn _assert_embedding<T: BaseEmbeddingService>() {}

        _assert_base_ai::<OpenAIClient>();
        _assert_embedding::<OpenAIClient>();
    }
}
