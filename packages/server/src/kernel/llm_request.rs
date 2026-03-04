// Simple completion traits for LLM text responses
//
// For structured JSON output, use `OpenAi::extract<T>()` from
// the ai-client crate with types that derive `JsonSchema + Deserialize`.

use ai_client::OpenAi;
use anyhow::Result;

/// Extension trait for simple text completions
#[async_trait::async_trait]
pub trait CompletionExt {
    /// Complete a prompt with an LLM (returns raw text response)
    async fn complete(&self, prompt: &str) -> Result<String>;
}

#[async_trait::async_trait]
impl CompletionExt for OpenAi {
    async fn complete(&self, prompt: &str) -> Result<String> {
        OpenAi::complete(self, prompt).await
    }
}

#[async_trait::async_trait]
impl CompletionExt for std::sync::Arc<OpenAi> {
    async fn complete(&self, prompt: &str) -> Result<String> {
        (**self).complete(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_ext_compiles() {
        fn _assert_completion_ext<T: CompletionExt>() {}

        _assert_completion_ext::<OpenAi>();
        _assert_completion_ext::<std::sync::Arc<OpenAi>>();
    }
}
