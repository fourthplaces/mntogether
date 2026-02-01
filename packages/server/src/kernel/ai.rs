// AI implementation using OpenAI
//
// This is the infrastructure implementation of BaseAI.
// Business logic (what to prompt for) lives in domain layers.

use anyhow::{Context, Result};
use async_trait::async_trait;
use rig::completion::Prompt;
use rig::providers::openai;
use serde::{Deserialize, Serialize};

use super::{BaseAI, BaseEmbeddingService};

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}

/// OpenAI implementation of AI capabilities
#[derive(Clone)]
pub struct OpenAIClient {
    client: openai::Client,
    api_key: String,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        let client = openai::Client::new(&api_key);
        Self { client, api_key }
    }

    /// Generate embeddings using OpenAI's text-embedding-ada-002 model
    pub async fn create_embedding(&self, text: &str) -> Result<EmbeddingResponse> {
        let http_client = reqwest::Client::new();

        let request = EmbeddingRequest {
            input: text.to_string(),
            model: "text-embedding-ada-002".to_string(),
        };

        let response = http_client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send embedding request to OpenAI")?;

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse embedding response")?;

        Ok(embedding_response)
    }
}

#[async_trait]
impl BaseAI for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.complete_with_model(prompt, None).await
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        self.complete_json_with_model(prompt, None).await
    }

    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let model_id = model.unwrap_or("gpt-4-turbo");

        tracing::debug!(
            prompt_length = prompt.len(),
            model = model_id,
            "Building OpenAI agent for completion"
        );

        // Build agent with the specified model
        let agent = match model_id {
            "gpt-5" => self.client.agent("gpt-5").preamble("You are a helpful assistant.").max_tokens(4096).build(),
            "gpt-4o" => self.client.agent(openai::GPT_4O).preamble("You are a helpful assistant.").max_tokens(4096).build(),
            "gpt-4-turbo" | _ => self.client.agent(openai::GPT_4_TURBO).preamble("You are a helpful assistant.").max_tokens(4096).build(),
        };

        tracing::info!(model = model_id, "Calling OpenAI API");

        let response = agent
            .prompt(prompt)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    model = model_id,
                    prompt_preview = %&prompt[..prompt.len().min(200)],
                    "OpenAI API call failed"
                );
                e
            })
            .context("Failed to call OpenAI API")?;

        tracing::info!(
            response_length = response.len(),
            model = model_id,
            "OpenAI API response received"
        );

        Ok(response)
    }

    async fn complete_json_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        // Same as complete_with_model for OpenAI
        self.complete_with_model(prompt, model).await
    }
}

#[async_trait]
impl BaseEmbeddingService for OpenAIClient {
    /// Generate embedding using OpenAI's text-embedding-ada-002 model
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        let response = self
            .create_embedding(text)
            .await
            .context("Failed to create embedding")?;

        let embedding = response
            .data
            .first()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))?
            .embedding
            .clone();

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_complete() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for integration tests");

        let client = OpenAIClient::new(api_key);

        let response = client
            .complete("Say 'Hello, World!' and nothing else.")
            .await
            .expect("AI completion should succeed");

        assert!(response.contains("Hello"));
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_embedding() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for integration tests");

        let client = OpenAIClient::new(api_key);

        let embedding = client
            .generate("Hello, world!")
            .await
            .expect("Embedding generation should succeed");

        assert_eq!(
            embedding.len(),
            1536,
            "OpenAI embeddings should be 1536 dimensions"
        );
    }
}
