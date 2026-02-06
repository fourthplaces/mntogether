// Fluent LLM request builder with automatic retry on parse failures
//
// Usage:
// ```rust
// let posts: Vec<Listing> = client
//     .request()
//     .system("You extract listings from websites")
//     .user(&format!("Extract from:\n{}", content))
//     .output::<Vec<Listing>>()
//     .await?;
// ```

use anyhow::{Context, Result};
use openai_client::{ChatRequest, Message, OpenAIClient};
use serde::de::DeserializeOwned;
use std::fmt::Write;

/// Builder for LLM requests with automatic JSON parsing and retry
pub struct LlmRequest<'a> {
    client: &'a OpenAIClient,
    system_prompt: Option<String>,
    user_message: Option<String>,
    max_retries: u32,
    /// Optional schema hint to include in retry prompts
    schema_hint: Option<String>,
    /// Optional model override (e.g., "gpt-5", "gpt-4-turbo")
    model: Option<String>,
}

impl<'a> LlmRequest<'a> {
    pub fn new(client: &'a OpenAIClient) -> Self {
        Self {
            client,
            system_prompt: None,
            user_message: None,
            max_retries: 3,
            schema_hint: None,
            model: None,
        }
    }

    /// Set the system prompt (instructions for the AI)
    pub fn system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the user message (the actual content/question)
    pub fn user(mut self, message: impl Into<String>) -> Self {
        self.user_message = Some(message.into());
        self
    }

    /// Set maximum retry attempts (default: 3)
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Provide a schema hint for retry error messages
    /// This helps the AI understand the expected structure
    pub fn schema_hint(mut self, hint: impl Into<String>) -> Self {
        self.schema_hint = Some(hint.into());
        self
    }

    /// Set the model to use (e.g., "gpt-5", "gpt-4-turbo")
    /// If not set, uses the default model configured in the AI client
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Execute the request and parse the response as JSON
    ///
    /// Automatically retries on parse failures, including the error message
    /// in subsequent attempts to help the AI fix its output.
    pub async fn output<T: DeserializeOwned>(self) -> Result<T> {
        let system = self.system_prompt.clone().unwrap_or_default();
        let user = self
            .user_message
            .clone()
            .ok_or_else(|| anyhow::anyhow!("User message is required"))?;

        let model = self.model.as_deref().unwrap_or("gpt-4o");
        let mut last_response = String::new();
        let mut last_error = String::new();

        for attempt in 1..=self.max_retries {
            let prompt = if attempt == 1 {
                self.build_initial_prompt(&system, &user)
            } else {
                self.build_retry_prompt(&last_response, &last_error)
            };

            tracing::info!(
                attempt,
                prompt_length = prompt.len(),
                model = model,
                "LLM request attempt"
            );

            let request = ChatRequest::new(model).message(Message::user(prompt));

            let response = self
                .client
                .chat_completion(request)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
                .context("LLM API call failed")?;

            last_response = response.content.clone();

            // Try to parse as JSON
            match serde_json::from_str::<T>(&response.content) {
                Ok(parsed) => {
                    tracing::info!(attempt, "Successfully parsed LLM response");
                    return Ok(parsed);
                }
                Err(e) => {
                    last_error = e.to_string();
                    tracing::warn!(
                        attempt,
                        error = %e,
                        response_preview = %response.content.chars().take(200).collect::<String>(),
                        "Failed to parse LLM response as JSON"
                    );

                    if attempt == self.max_retries {
                        return Err(anyhow::anyhow!(
                            "Failed to get valid JSON after {} attempts. Last error: {}",
                            self.max_retries,
                            e
                        ));
                    }
                }
            }
        }

        unreachable!()
    }

    /// Execute the request and return raw text (no parsing)
    pub async fn text(self) -> Result<String> {
        let system = self.system_prompt.clone().unwrap_or_default();
        let user = self
            .user_message
            .clone()
            .ok_or_else(|| anyhow::anyhow!("User message is required"))?;

        let model = self.model.as_deref().unwrap_or("gpt-4o");
        let prompt = self.build_initial_prompt(&system, &user);

        let request = ChatRequest::new(model).message(Message::user(prompt));

        let response = self
            .client
            .chat_completion(request)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(response.content)
    }

    fn build_initial_prompt(&self, system: &str, user: &str) -> String {
        let mut prompt = String::new();

        if !system.is_empty() {
            let _ = writeln!(prompt, "{}\n", system);
        }

        let _ = writeln!(prompt, "{}", user);

        // Add JSON output instructions - be extremely explicit
        let _ = writeln!(
            prompt,
            r#"

CRITICAL: Your response will be parsed directly by a JSON parser.

DO NOT include:
- Markdown code fences (```)
- The word "json" before the data
- Any text before the opening [ or {{
- Any text after the closing ] or }}
- Explanations or commentary

DO:
- Start your response with [ or {{ immediately
- End your response with ] or }} immediately
- Return syntactically valid JSON only"#
        );

        prompt
    }

    fn build_retry_prompt(&self, last_response: &str, error: &str) -> String {
        let response_preview: String = last_response.chars().take(500).collect();

        let mut prompt = format!(
            r#"JSON PARSE FAILED. Your previous response could not be parsed.

ERROR: {error}

Your response was:
{response_preview}

This failed because your response is not valid JSON.
"#
        );

        // Include schema hint if provided
        if let Some(hint) = &self.schema_hint {
            let _ = writeln!(prompt, "\nEXPECTED FORMAT:\n{}", hint);
        }

        let _ = writeln!(
            prompt,
            r#"
RESPOND WITH RAW JSON ONLY:
- First character must be [ or {{
- Last character must be ] or }}
- No ``` markdown fences
- No "json" prefix
- No explanation text
- Properly escape special characters in strings
- Use null for missing values, not undefined"#
        );

        prompt
    }
}

/// Extension trait to add fluent request builder to OpenAIClient
pub trait LlmRequestExt {
    fn request(&self) -> LlmRequest<'_>;
}

impl LlmRequestExt for OpenAIClient {
    fn request(&self) -> LlmRequest<'_> {
        LlmRequest::new(self)
    }
}

// Also implement for Arc<OpenAIClient>
impl LlmRequestExt for std::sync::Arc<OpenAIClient> {
    fn request(&self) -> LlmRequest<'_> {
        LlmRequest::new(self)
    }
}

/// Extension trait for simple completions (backwards compatibility)
#[async_trait::async_trait]
pub trait CompletionExt {
    /// Complete a prompt with an LLM (returns raw text response)
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Complete a prompt expecting JSON response (returns raw JSON string)
    async fn complete_json(&self, prompt: &str) -> Result<String>;

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

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        self.complete(prompt).await
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

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        (**self).complete_json(prompt).await
    }

    async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        (**self).complete_with_model(prompt, model).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_request_compiles() {
        // Just verify the types are correct
        fn _assert_request_ext<T: LlmRequestExt>() {}
        fn _assert_completion_ext<T: CompletionExt>() {}

        _assert_request_ext::<OpenAIClient>();
        _assert_completion_ext::<OpenAIClient>();
        _assert_request_ext::<std::sync::Arc<OpenAIClient>>();
        _assert_completion_ext::<std::sync::Arc<OpenAIClient>>();
    }
}
