//! OpenAI API request and response types.

use serde::{Deserialize, Serialize};

// =============================================================================
// Chat Completion
// =============================================================================

/// Chat completion request.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// Model to use (e.g., "gpt-4o", "gpt-4o-mini")
    pub model: String,

    /// Conversation messages
    pub messages: Vec<Message>,

    /// Sampling temperature (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens in completion (for older models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Maximum completion tokens (for o1, o3, gpt-5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
        }
    }
}

impl ChatRequest {
    /// Create a new chat request with the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Add a message to the conversation.
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens (for older models).
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set max completion tokens (for newer models).
    pub fn max_completion_tokens(mut self, max_completion_tokens: u32) -> Self {
        self.max_completion_tokens = Some(max_completion_tokens);
        self
    }

    /// Check if a model requires max_completion_tokens instead of max_tokens.
    pub fn uses_max_completion_tokens(model: &str) -> bool {
        model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("gpt-5")
            || model.contains("-o1")
            || model.contains("-o3")
    }
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "system", "user", "assistant"
    pub role: String,

    /// Message content
    pub content: String,
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Chat completion response.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// Response content
    pub content: String,

    /// Token usage statistics
    pub usage: Option<Usage>,
}

/// Raw chat response from API (for internal parsing).
#[derive(Debug, Deserialize)]
pub(crate) struct ChatResponseRaw {
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatChoice {
    pub message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatMessageResponse {
    pub content: String,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,

    /// Tokens in the completion
    pub completion_tokens: u32,

    /// Total tokens used
    pub total_tokens: u32,
}

// =============================================================================
// Structured Output
// =============================================================================

/// Structured output request with JSON schema.
#[derive(Debug, Serialize)]
pub struct StructuredRequest {
    /// Model to use
    pub model: String,

    /// Conversation messages
    pub messages: Vec<Message>,

    /// Temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Response format with JSON schema
    pub response_format: ResponseFormat,
}

impl StructuredRequest {
    /// Create a new structured request.
    pub fn new(
        model: impl Into<String>,
        system: impl Into<String>,
        user: impl Into<String>,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            model: model.into(),
            messages: vec![
                Message::system(system),
                Message::user(user),
            ],
            temperature: Some(0.0),
            response_format: ResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: JsonSchemaFormat {
                    name: "structured_response".to_string(),
                    strict: true,
                    schema,
                },
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    pub json_schema: JsonSchemaFormat,
}

#[derive(Debug, Serialize)]
pub struct JsonSchemaFormat {
    pub name: String,
    pub strict: bool,
    pub schema: serde_json::Value,
}

// =============================================================================
// Function Calling
// =============================================================================

/// Function calling request.
#[derive(Debug, Serialize)]
pub struct FunctionRequest {
    /// Model to use
    pub model: String,

    /// Conversation messages
    pub messages: Vec<serde_json::Value>,

    /// Tool definitions
    pub tools: serde_json::Value,

    /// Tool choice strategy
    pub tool_choice: String,
}

impl FunctionRequest {
    /// Create a new function request with auto tool choice.
    pub fn new(
        model: impl Into<String>,
        messages: Vec<serde_json::Value>,
        tools: serde_json::Value,
    ) -> Self {
        Self {
            model: model.into(),
            messages,
            tools,
            tool_choice: "auto".to_string(),
        }
    }
}

/// Function calling response.
#[derive(Debug)]
pub struct FunctionResponse {
    /// Assistant message with tool calls or content
    pub message: serde_json::Value,
}

// =============================================================================
// Embeddings
// =============================================================================

/// Embedding request.
#[derive(Debug, Serialize)]
pub(crate) struct EmbeddingRequest {
    /// Model to use (e.g., "text-embedding-3-small")
    pub model: String,

    /// Text to embed
    pub input: String,
}

/// Embedding response.
#[derive(Debug, Deserialize)]
pub(crate) struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EmbeddingData {
    pub embedding: Vec<f32>,
}

// =============================================================================
// Utilities
// =============================================================================

/// Truncate a string to at most `max_bytes` bytes at a character boundary.
pub fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

/// Strip markdown code blocks from a response.
pub fn strip_code_blocks(response: &str) -> &str {
    response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let sys = Message::system("You are helpful");
        assert_eq!(sys.role, "system");

        let user = Message::user("Hello");
        assert_eq!(user.role, "user");

        let assistant = Message::assistant("Hi there");
        assert_eq!(assistant.role, "assistant");
    }

    #[test]
    fn test_chat_request_builder() {
        let req = ChatRequest::new("gpt-4o")
            .message(Message::user("Hello"))
            .temperature(0.7)
            .max_tokens(100);

        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.max_tokens, Some(100));
    }

    #[test]
    fn test_uses_max_completion_tokens() {
        assert!(ChatRequest::uses_max_completion_tokens("o1-preview"));
        assert!(ChatRequest::uses_max_completion_tokens("o3-mini"));
        assert!(ChatRequest::uses_max_completion_tokens("gpt-5-turbo"));
        assert!(!ChatRequest::uses_max_completion_tokens("gpt-4o"));
        assert!(!ChatRequest::uses_max_completion_tokens("gpt-4"));
    }

    #[test]
    fn test_truncate_to_char_boundary() {
        let text = "Hello 世界";
        let truncated = truncate_to_char_boundary(text, 8);
        assert!(truncated.len() <= 8);
        assert!(text.starts_with(truncated));
    }

    #[test]
    fn test_strip_code_blocks() {
        assert_eq!(strip_code_blocks("```json\n{}\n```"), "{}");
        assert_eq!(strip_code_blocks("```\n{}\n```"), "{}");
        assert_eq!(strip_code_blocks("{}"), "{}");
    }
}
