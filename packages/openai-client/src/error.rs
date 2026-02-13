//! Error types for OpenAI client.

use thiserror::Error;

/// Result type for OpenAI client operations.
pub type Result<T> = std::result::Result<T, OpenAIError>;

/// OpenAI client errors.
#[derive(Debug, Error)]
pub enum OpenAIError {
    /// Configuration error (missing API key, invalid settings)
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network error (connection failed, timeout)
    #[error("Network error: {0}")]
    Network(String),

    /// API error (non-2xx response, rate limit, invalid request)
    #[error("API error: {0}")]
    Api(String),

    /// Parse error (invalid JSON, unexpected response format)
    #[error("Parse error: {0}")]
    Parse(String),
}
