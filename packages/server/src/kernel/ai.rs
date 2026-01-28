// AI implementation using Anthropic Claude
//
// This is the infrastructure implementation of BaseAI.
// Business logic (what to prompt for) lives in domain layers.

use anyhow::{Context, Result};
use async_trait::async_trait;
use rig::completion::Prompt;
use rig::providers::anthropic;

use super::BaseAI;

/// Anthropic Claude implementation of AI capabilities
pub struct ClaudeClient {
    client: anthropic::Client,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        let client = anthropic::ClientBuilder::new(&api_key).build();
        Self { client }
    }
}

#[async_trait]
impl BaseAI for ClaudeClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let agent = self
            .client
            .agent(anthropic::CLAUDE_3_5_SONNET)
            .preamble("You are a helpful assistant.")
            .max_tokens(4096)
            .build();

        let response = agent
            .prompt(prompt)
            .await
            .context("Failed to call Anthropic API")?;

        Ok(response)
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        // Same as complete for Claude
        self.complete(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_complete() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set for integration tests");

        let client = ClaudeClient::new(api_key);

        let response = client
            .complete("Say 'Hello, World!' and nothing else.")
            .await
            .expect("AI completion should succeed");

        assert!(response.contains("Hello"));
    }
}
