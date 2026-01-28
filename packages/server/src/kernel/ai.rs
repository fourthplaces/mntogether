// AI implementation using OpenAI
//
// This is the infrastructure implementation of BaseAI.
// Business logic (what to prompt for) lives in domain layers.

use anyhow::{Context, Result};
use async_trait::async_trait;
use rig::completion::Prompt;
use rig::providers::openai;

use super::BaseAI;

/// OpenAI implementation of AI capabilities
pub struct OpenAIClient {
    client: openai::Client,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        let client = openai::Client::new(&api_key);
        Self { client }
    }
}

#[async_trait]
impl BaseAI for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let agent = self
            .client
            .agent("gpt-4o")
            .preamble("You are a helpful assistant.")
            .build();

        let response = agent
            .prompt(prompt)
            .await
            .context("Failed to call OpenAI API")?;

        Ok(response)
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        // Same as complete for OpenAI
        self.complete(prompt).await
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
}
