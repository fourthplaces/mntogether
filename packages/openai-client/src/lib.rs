//! Pure OpenAI REST API client
//!
//! A clean, minimal client for the OpenAI API with no domain-specific logic.
//! Supports chat completions, embeddings, structured outputs, and function calling.
//!
//! # Example
//!
//! ```rust,ignore
//! use openai_client::{OpenAIClient, ChatRequest, Message};
//!
//! let client = OpenAIClient::from_env()?;
//!
//! // Chat completion
//! let response = client.chat_completion(ChatRequest {
//!     model: "gpt-4o".into(),
//!     messages: vec![Message::user("Hello!")],
//!     ..Default::default()
//! }).await?;
//!
//! // Embeddings
//! let embedding = client.create_embedding("text to embed", "text-embedding-3-small").await?;
//! ```
//!
//! # Type-Safe Structured Output
//!
//! ```rust,ignore
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct Post {
//!     title: String,
//!     description: String,
//! }
//!
//! // Schema generated automatically from type!
//! let posts: Vec<Post> = client
//!     .extract::<Vec<Post>>("gpt-4o", system_prompt, user_prompt)
//!     .await?;
//! ```
//!
//! # Agent with Tools
//!
//! ```rust,ignore
//! let response = client
//!     .agent("gpt-4o")
//!     .system("You are a research assistant")
//!     .tool(WebSearch)
//!     .build()
//!     .chat("Find info about Rust")
//!     .await?;
//! ```

pub mod agent;
pub mod error;
pub mod schema;
pub mod streaming;
pub mod tool;
pub mod types;

pub use agent::{Agent, AgentBuilder, AgentResponse};
pub use error::{OpenAIError, Result};
pub use schema::StructuredOutput;
pub use streaming::ChatCompletionStream;
pub use tool::{ErasedTool, Tool, ToolCall, ToolDefinition, ToolError};
pub use types::*;

use reqwest::Client;
use tracing::{debug, warn};

/// Pure OpenAI API client.
#[derive(Clone)]
pub struct OpenAIClient {
    http_client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    /// Create a new OpenAI client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http_client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| OpenAIError::Config("OPENAI_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    /// Set a custom base URL (for Azure, proxies, etc.).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Create an agent builder with the specified model.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client
    ///     .agent("gpt-4o")
    ///     .system("You are a helpful assistant")
    ///     .tool(MyTool)
    ///     .build()
    ///     .chat("Hello!")
    ///     .await?;
    /// ```
    pub fn agent(&self, model: impl Into<String>) -> AgentBuilder<'_> {
        AgentBuilder::new(self, model)
    }

    /// Type-safe structured output extraction.
    ///
    /// Automatically generates a JSON schema from the type `T` using `schemars`,
    /// sends it to OpenAI, and deserializes the response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use schemars::JsonSchema;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, JsonSchema)]
    /// struct Post {
    ///     title: String,
    ///     description: String,
    /// }
    ///
    /// #[derive(Deserialize, JsonSchema)]
    /// struct Response {
    ///     posts: Vec<Post>,
    /// }
    ///
    /// let result: Response = client
    ///     .extract::<Response>("gpt-4o", system_prompt, user_prompt)
    ///     .await?;
    /// ```
    pub async fn extract<T: StructuredOutput>(
        &self,
        model: &str,
        system_prompt: impl Into<String>,
        user_prompt: impl Into<String>,
    ) -> Result<T> {
        let schema = T::openai_schema();

        debug!(
            type_name = T::type_name(),
            schema = %serde_json::to_string_pretty(&schema).unwrap_or_default(),
            "Generated OpenAI schema for extraction"
        );

        let request = StructuredRequest::new(model, system_prompt, user_prompt, schema);
        let json_str = self.structured_output(request).await?;

        serde_json::from_str(&json_str).map_err(|e| {
            OpenAIError::Parse(format!("Failed to deserialize response: {}", e))
        })
    }

    /// Streaming chat completion.
    ///
    /// Send messages and get a stream of token chunks back.
    /// Uses SSE (server-sent events) from the OpenAI API.
    pub async fn chat_completion_stream(
        &self,
        request: ChatRequest,
    ) -> Result<streaming::ChatCompletionStream> {
        use reqwest::header;

        // Build JSON body with stream: true
        let mut body = serde_json::to_value(&request)
            .map_err(|e| OpenAIError::Parse(format!("Failed to serialize request: {}", e)))?;
        body["stream"] = serde_json::Value::Bool(true);

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "OpenAI streaming request failed");
                OpenAIError::Network(e.to_string())
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(status = %status, error = %error_text, "OpenAI streaming API error");
            return Err(OpenAIError::Api(format!(
                "OpenAI streaming API error: {}",
                error_text
            )));
        }

        Ok(streaming::ChatCompletionStream::new(response.bytes_stream()))
    }

    /// Chat completion.
    ///
    /// Send messages to the chat completion API and get a response.
    pub async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        let start = std::time::Instant::now();

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "OpenAI request failed");
                OpenAIError::Network(e.to_string())
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(status = %status, error = %error_text, "OpenAI API error");
            return Err(OpenAIError::Api(format!("OpenAI API error: {}", error_text)));
        }

        let chat_response: types::ChatResponseRaw = response
            .json()
            .await
            .map_err(|e| OpenAIError::Parse(e.to_string()))?;

        let content = chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| OpenAIError::Api("No response from OpenAI".into()))?;

        debug!(
            model = %request.model,
            duration_ms = start.elapsed().as_millis(),
            "OpenAI chat completion"
        );

        Ok(ChatResponse {
            content,
            usage: chat_response.usage,
        })
    }

    /// Structured output with JSON schema.
    ///
    /// Uses OpenAI's `json_schema` response format for guaranteed valid JSON.
    pub async fn structured_output(&self, request: StructuredRequest) -> Result<String> {
        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenAIError::Api(format!("OpenAI structured output error: {}", error_text)));
        }

        let chat_response: types::ChatResponseRaw = response
            .json()
            .await
            .map_err(|e| OpenAIError::Parse(e.to_string()))?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| OpenAIError::Api("No response from OpenAI".into()))
    }

    /// Function calling (tool use).
    ///
    /// Send messages with tool definitions and get tool calls or content back.
    pub async fn function_calling(&self, request: FunctionRequest) -> Result<FunctionResponse> {
        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenAIError::Api(format!("OpenAI tools API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenAIError::Parse(e.to_string()))?;

        Ok(FunctionResponse {
            message: response_json["choices"][0]["message"].clone(),
        })
    }

    /// Create embedding for text.
    ///
    /// Returns a vector (typically 1536 dimensions for text-embedding-3-small).
    pub async fn create_embedding(&self, text: &str, model: &str) -> Result<Vec<f32>> {
        let request = types::EmbeddingRequest {
            model: model.to_string(),
            input: text.to_string(),
        };

        let response = self
            .http_client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "Embedding request failed");
                OpenAIError::Network(e.to_string())
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(error = %error_text, "OpenAI embedding error");
            return Err(OpenAIError::Api(format!("OpenAI embedding error: {}", error_text)));
        }

        let embed_response: types::EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| OpenAIError::Parse(e.to_string()))?;

        embed_response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| OpenAIError::Api("No embedding from OpenAI".into()))
    }

    /// Create embeddings for multiple texts (batch operation).
    pub async fn create_embeddings_batch(&self, texts: &[&str], model: &str) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.create_embedding(text, model).await?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = OpenAIClient::new("sk-test")
            .with_base_url("https://custom.api.com");

        assert_eq!(client.api_key, "sk-test");
        assert_eq!(client.base_url, "https://custom.api.com");
    }
}
