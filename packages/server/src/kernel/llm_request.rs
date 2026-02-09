// Simple completion traits for LLM text responses
//
// For structured JSON output, use `OpenAIClient::extract<T>()` from
// the openai-client crate with types that derive `JsonSchema + Deserialize`.

use anyhow::Result;
use openai_client::{ChatRequest, Message, OpenAIClient};

/// Extension trait for simple text completions
#[async_trait::async_trait]
pub trait CompletionExt {
    /// Complete a prompt with an LLM (returns raw text response)
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Complete a prompt with a specific model
    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String>;
}

#[async_trait::async_trait]
impl CompletionExt for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let request = ChatRequest::new("gpt-4o").message(Message::user(prompt));

        let response = self
            .chat_completion(request)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(response.content)
    }

    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let model = model.unwrap_or("gpt-4o");
        let request = ChatRequest::new(model).message(Message::user(prompt));

        let response = self
            .chat_completion(request)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(response.content)
    }
}

#[async_trait::async_trait]
impl CompletionExt for std::sync::Arc<OpenAIClient> {
    async fn complete(&self, prompt: &str) -> Result<String> {
        (**self).complete(prompt).await
    }

    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        (**self).complete_with_model(prompt, model).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_ext_compiles() {
        fn _assert_completion_ext<T: CompletionExt>() {}

        _assert_completion_ext::<OpenAIClient>();
        _assert_completion_ext::<std::sync::Arc<OpenAIClient>>();
    }
}
