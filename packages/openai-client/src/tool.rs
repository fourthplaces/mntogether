//! Tool calling traits and types for OpenAI function calling.
//!
//! Provides a type-safe, ergonomic API for defining tools that can be called by the model.
//!
//! # Example
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//! use openai_client::Tool;
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct SearchArgs {
//!     query: String,
//! }
//!
//! struct WebSearch;
//!
//! #[async_trait]
//! impl Tool for WebSearch {
//!     const NAME: &'static str = "web_search";
//!     type Args = SearchArgs;
//!     type Output = Vec<String>;
//!     type Error = anyhow::Error;
//!
//!     fn description(&self) -> &str {
//!         "Search the web for information"
//!     }
//!
//!     async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
//!         // Implementation
//!         Ok(vec![format!("Results for: {}", args.query)])
//!     }
//! }
//! ```

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::schema::StructuredOutput;

/// A tool that can be called by the OpenAI model.
///
/// Tools have typed arguments and outputs, with automatic schema generation.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The unique name of this tool.
    const NAME: &'static str;

    /// The argument type for this tool (must derive `Deserialize` and `JsonSchema`).
    type Args: DeserializeOwned + JsonSchema + Send;

    /// The output type for this tool (must derive `Serialize`).
    type Output: Serialize + Send;

    /// The error type for this tool.
    type Error: std::error::Error + Send + Sync + 'static;

    /// A description of what this tool does.
    fn description(&self) -> &str;

    /// Execute the tool with the given arguments.
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>;

    /// Generate the OpenAI tool definition for this tool.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: self.description().to_string(),
            parameters: Self::Args::openai_schema(),
        }
    }
}

/// OpenAI tool definition format.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    /// The name of the tool.
    pub name: String,

    /// A description of what the tool does.
    pub description: String,

    /// JSON schema for the tool's parameters.
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Convert to OpenAI API format.
    pub fn to_openai_format(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters
            }
        })
    }
}

/// A tool call from the model.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// The ID of this tool call (for matching responses).
    pub id: String,

    /// The name of the tool to call.
    pub name: String,

    /// The arguments as a JSON string.
    pub arguments: String,
}

impl ToolCall {
    /// Parse a tool call from OpenAI's response format.
    pub fn from_openai_value(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: value.get("id")?.as_str()?.to_string(),
            name: value.get("function")?.get("name")?.as_str()?.to_string(),
            arguments: value.get("function")?.get("arguments")?.as_str()?.to_string(),
        })
    }

    /// Parse arguments into a typed struct.
    pub fn parse_args<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.arguments)
    }
}

/// Type-erased tool for storing heterogeneous tools in collections.
///
/// This allows storing different tool types in the same `Vec<Box<dyn ErasedTool>>`.
#[async_trait]
pub trait ErasedTool: Send + Sync {
    /// Get the tool's name.
    fn name(&self) -> &str;

    /// Get the tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with JSON arguments, returning JSON output.
    async fn call_erased(&self, arguments: &str) -> Result<String, ToolError>;
}

/// Error type for erased tool calls.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Failed to parse tool arguments.
    #[error("Failed to parse arguments: {0}")]
    ArgumentParse(String),

    /// Tool execution failed.
    #[error("Tool execution failed: {0}")]
    Execution(String),

    /// Failed to serialize tool output.
    #[error("Failed to serialize output: {0}")]
    OutputSerialize(String),
}

/// Blanket implementation of `ErasedTool` for all `Tool` implementors.
#[async_trait]
impl<T: Tool> ErasedTool for T {
    fn name(&self) -> &str {
        T::NAME
    }

    fn definition(&self) -> ToolDefinition {
        Tool::definition(self)
    }

    async fn call_erased(&self, arguments: &str) -> Result<String, ToolError> {
        // Parse arguments
        let args: T::Args = serde_json::from_str(arguments)
            .map_err(|e| ToolError::ArgumentParse(e.to_string()))?;

        // Execute tool
        let output = self
            .call(args)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        // Serialize output
        serde_json::to_string(&output).map_err(|e| ToolError::OutputSerialize(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, JsonSchema)]
    struct EchoArgs {
        message: String,
    }

    #[derive(Serialize)]
    struct EchoOutput {
        echoed: String,
    }

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        const NAME: &'static str = "echo";
        type Args = EchoArgs;
        type Output = EchoOutput;
        type Error = std::convert::Infallible;

        fn description(&self) -> &str {
            "Echo back the input message"
        }

        async fn call(&self, args: Self::Args) -> std::result::Result<Self::Output, Self::Error> {
            Ok(EchoOutput {
                echoed: args.message,
            })
        }
    }

    #[test]
    fn test_tool_definition() {
        let tool = EchoTool;
        let def = Tool::definition(&tool);

        assert_eq!(def.name, "echo");
        assert_eq!(def.description, "Echo back the input message");
        assert!(def.parameters.is_object());
    }

    #[test]
    fn test_tool_definition_openai_format() {
        let tool = EchoTool;
        let def = Tool::definition(&tool);
        let openai_format = def.to_openai_format();

        assert_eq!(openai_format["type"], "function");
        assert_eq!(openai_format["function"]["name"], "echo");
    }

    #[test]
    fn test_tool_call_parsing() {
        let value = serde_json::json!({
            "id": "call_123",
            "function": {
                "name": "echo",
                "arguments": "{\"message\": \"hello\"}"
            }
        });

        let call = ToolCall::from_openai_value(&value).unwrap();
        assert_eq!(call.id, "call_123");
        assert_eq!(call.name, "echo");

        let args: EchoArgs = call.parse_args().unwrap();
        assert_eq!(args.message, "hello");
    }

    #[tokio::test]
    async fn test_erased_tool() {
        let tool: Box<dyn ErasedTool> = Box::new(EchoTool);

        assert_eq!(tool.name(), "echo");

        let result = tool.call_erased(r#"{"message": "test"}"#).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["echoed"], "test");
    }
}
