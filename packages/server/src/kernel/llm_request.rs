// Fluent LLM request builder with automatic retry on parse failures
//
// Usage:
// ```rust
// let posts: Vec<Listing> = ai
//     .request()
//     .system("You extract listings from websites")
//     .user(&format!("Extract from:\n{}", content))
//     .output::<Vec<Listing>>()
//     .await?;
// ```

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::fmt::Write;

use super::BaseAI;

/// Builder for LLM requests with automatic JSON parsing and retry
pub struct LlmRequest<'a> {
    ai: &'a dyn BaseAI,
    system_prompt: Option<String>,
    user_message: Option<String>,
    max_retries: u32,
    /// Optional schema hint to include in retry prompts
    schema_hint: Option<String>,
    /// Optional model override (e.g., "gpt-5", "gpt-4-turbo")
    model: Option<String>,
}

impl<'a> LlmRequest<'a> {
    pub fn new(ai: &'a dyn BaseAI) -> Self {
        Self {
            ai,
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
                model = ?self.model,
                "LLM request attempt"
            );

            let response = self
                .ai
                .complete_json_with_model(&prompt, self.model.as_deref())
                .await
                .context("LLM API call failed")?;

            last_response = response.clone();

            // Try to parse as JSON
            match serde_json::from_str::<T>(&response) {
                Ok(parsed) => {
                    tracing::info!(attempt, "Successfully parsed LLM response");
                    return Ok(parsed);
                }
                Err(e) => {
                    last_error = e.to_string();
                    tracing::warn!(
                        attempt,
                        error = %e,
                        response_preview = %response.chars().take(200).collect::<String>(),
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

        let prompt = self.build_initial_prompt(&system, &user);
        self.ai
            .complete_with_model(&prompt, self.model.as_deref())
            .await
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

/// Extension trait to add fluent request builder to BaseAI
///
/// Works with both concrete types and trait objects via the blanket impl.
pub trait LlmRequestExt {
    fn request(&self) -> LlmRequest<'_>;
}

impl<T: BaseAI> LlmRequestExt for T {
    fn request(&self) -> LlmRequest<'_> {
        LlmRequest::new(self)
    }
}

// Also implement for trait objects explicitly (with lifetime bounds)
impl LlmRequestExt for dyn BaseAI + '_ {
    fn request(&self) -> LlmRequest<'_> {
        LlmRequest::new(self)
    }
}

impl LlmRequestExt for dyn BaseAI + Send + '_ {
    fn request(&self) -> LlmRequest<'_> {
        LlmRequest::new(self)
    }
}

impl LlmRequestExt for dyn BaseAI + Send + Sync + '_ {
    fn request(&self) -> LlmRequest<'_> {
        LlmRequest::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestOutput {
        name: String,
        count: i32,
    }

    struct MockAI {
        responses: Vec<String>,
        call_count: Arc<AtomicU32>,
    }

    impl MockAI {
        fn new(responses: Vec<&str>) -> Self {
            Self {
                responses: responses.into_iter().map(String::from).collect(),
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    #[async_trait]
    impl BaseAI for MockAI {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst) as usize;
            Ok(self.responses.get(idx).cloned().unwrap_or_default())
        }
    }

    #[tokio::test]
    async fn test_successful_first_attempt() {
        let ai = MockAI::new(vec![r#"{"name": "test", "count": 42}"#]);

        let result: TestOutput = ai
            .request()
            .system("You are helpful")
            .user("Give me data")
            .output()
            .await
            .unwrap();

        assert_eq!(result.name, "test");
        assert_eq!(result.count, 42);
        assert_eq!(ai.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_on_invalid_json() {
        let ai = MockAI::new(vec![
            "```json\n{\"name\": \"test\"}\n```", // Invalid (markdown)
            r#"{"name": "test", "count": 42}"#,   // Valid
        ]);

        let result: TestOutput = ai
            .request()
            .user("Give me data")
            .max_retries(3)
            .output()
            .await
            .unwrap();

        assert_eq!(result.name, "test");
        assert_eq!(ai.call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_fails_after_max_retries() {
        let ai = MockAI::new(vec!["not json", "still not json", "definitely not json"]);

        let result: Result<TestOutput> = ai
            .request()
            .user("Give me data")
            .max_retries(3)
            .output()
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to get valid JSON after 3 attempts"));
    }
}
