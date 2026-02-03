//! Bridge between extraction library's OpenAI and server's AI traits.
//!
//! The extraction library's `OpenAI` struct implements the extraction-specific
//! `AI` trait. This module implements the server's `BaseAI` and
//! `BaseEmbeddingService` traits for `OpenAI`, making it the unified AI
//! implementation across both layers.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    extraction::ai::OpenAI                            │
//! │  - THE unified AI implementation                                     │
//! │  - Implements both extraction::AI AND server::BaseAI                 │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use anyhow::{Context, Result};
use async_trait::async_trait;
use extraction::ai::OpenAI as ExtractionOpenAI;

use super::{BaseAI, BaseEmbeddingService};

/// Wrapper around extraction's OpenAI that implements server traits.
///
/// This allows using the extraction library's OpenAI as the server's
/// AI and embedding service without code duplication.
#[derive(Clone)]
pub struct ExtractionAIBridge {
    openai: ExtractionOpenAI,
}

impl ExtractionAIBridge {
    /// Create a new bridge from an existing extraction OpenAI instance.
    pub fn new(openai: ExtractionOpenAI) -> Self {
        Self { openai }
    }

    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let openai = ExtractionOpenAI::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
        Ok(Self::new(openai))
    }

    /// Get a reference to the underlying OpenAI instance.
    pub fn inner(&self) -> &ExtractionOpenAI {
        &self.openai
    }

    /// Get the underlying OpenAI instance.
    pub fn into_inner(self) -> ExtractionOpenAI {
        self.openai
    }
}

#[async_trait]
impl BaseAI for ExtractionAIBridge {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.openai
            .complete(prompt)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        // Same as complete - the model returns JSON when asked
        self.complete(prompt).await
    }

    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        self.openai
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
        self.openai
            .generate_structured(system_prompt, user_prompt, schema)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn generate_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.openai
            .generate_with_tools(messages, tools)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[async_trait]
impl BaseEmbeddingService for ExtractionAIBridge {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        use extraction::AI;
        self.openai
            .embed(text)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to generate embedding")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_compiles() {
        // Just verify the types are correct
        fn _assert_base_ai<T: BaseAI>() {}
        fn _assert_embedding<T: BaseEmbeddingService>() {}

        _assert_base_ai::<ExtractionAIBridge>();
        _assert_embedding::<ExtractionAIBridge>();
    }
}
