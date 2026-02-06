//! Agent with automatic tool calling loop.
//!
//! Provides a high-level API for building AI agents that can use tools.
//!
//! # Example
//!
//! ```rust,ignore
//! use openai_client::{OpenAIClient, Tool};
//!
//! let response = client
//!     .agent("gpt-4o")
//!     .system("You are a research assistant")
//!     .tool(WebSearch)
//!     .tool(Calculator)
//!     .max_iterations(5)
//!     .build()
//!     .chat("What is the population of Tokyo?")
//!     .await?;
//! ```

use crate::tool::{ErasedTool, Tool, ToolCall};
use crate::{OpenAIClient, OpenAIError, Result};
use tracing::{debug, info, warn};

/// Builder for creating an Agent.
pub struct AgentBuilder<'a> {
    client: &'a OpenAIClient,
    model: String,
    system_prompt: Option<String>,
    tools: Vec<Box<dyn ErasedTool>>,
    max_iterations: usize,
    temperature: Option<f32>,
}

impl<'a> AgentBuilder<'a> {
    /// Create a new agent builder.
    pub(crate) fn new(client: &'a OpenAIClient, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            system_prompt: None,
            tools: Vec::new(),
            max_iterations: 10,
            temperature: None,
        }
    }

    /// Set the system prompt.
    pub fn system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a tool to the agent.
    pub fn tool<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    /// Set the maximum number of tool-calling iterations.
    ///
    /// Default is 10. The agent will stop after this many iterations
    /// even if the model keeps requesting tool calls.
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set the temperature for generation.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Build the agent.
    pub fn build(self) -> Agent<'a> {
        Agent {
            client: self.client,
            model: self.model,
            system_prompt: self.system_prompt,
            tools: self.tools,
            max_iterations: self.max_iterations,
            temperature: self.temperature,
        }
    }
}

/// An AI agent that can use tools to accomplish tasks.
pub struct Agent<'a> {
    client: &'a OpenAIClient,
    model: String,
    system_prompt: Option<String>,
    tools: Vec<Box<dyn ErasedTool>>,
    max_iterations: usize,
    temperature: Option<f32>,
}

/// Response from an agent chat.
#[derive(Debug)]
pub struct AgentResponse {
    /// The final text response from the agent.
    pub content: String,

    /// The tool calls that were made during the conversation.
    pub tool_calls_made: Vec<String>,

    /// Number of iterations (API calls) made.
    pub iterations: usize,
}

impl<'a> Agent<'a> {
    /// Send a message to the agent and get a response.
    ///
    /// This method handles the tool-calling loop automatically:
    /// 1. Send the user message to the model
    /// 2. If the model requests tool calls, execute them
    /// 3. Send tool results back to the model
    /// 4. Repeat until the model responds with text or max iterations reached
    pub async fn chat(&self, user_message: impl Into<String>) -> Result<AgentResponse> {
        let user_message = user_message.into();
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Add system message if present
        if let Some(ref system) = self.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        // Add user message
        messages.push(serde_json::json!({
            "role": "user",
            "content": user_message
        }));

        self.run_tool_loop(messages).await
    }

    /// Same as `chat()` but accepts pre-built message history.
    ///
    /// If a system prompt was set on the builder and the first message in
    /// `messages` is not already a system message, it will be prepended.
    pub async fn chat_with_history(
        &self,
        mut messages: Vec<serde_json::Value>,
    ) -> Result<AgentResponse> {
        // Prepend system prompt if set and not already present
        if let Some(ref system) = self.system_prompt {
            let has_system = messages
                .first()
                .and_then(|m| m.get("role"))
                .and_then(|r| r.as_str())
                == Some("system");

            if !has_system {
                messages.insert(
                    0,
                    serde_json::json!({
                        "role": "system",
                        "content": system
                    }),
                );
            }
        }

        self.run_tool_loop(messages).await
    }

    /// Core tool-calling loop shared by `chat()` and `chat_with_history()`.
    async fn run_tool_loop(&self, mut messages: Vec<serde_json::Value>) -> Result<AgentResponse> {
        let mut tool_calls_made = Vec::new();
        let mut iterations = 0;

        // Build tool definitions
        let tool_defs: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|t| t.definition().to_openai_format())
            .collect();

        loop {
            iterations += 1;

            if iterations > self.max_iterations {
                warn!(
                    max_iterations = self.max_iterations,
                    "Agent reached max iterations"
                );
                return Err(OpenAIError::Api(format!(
                    "Agent reached max iterations ({})",
                    self.max_iterations
                )));
            }

            info!(
                iteration = iterations,
                model = %self.model,
                message_count = messages.len(),
                tool_count = self.tools.len(),
                "Agent iteration starting"
            );

            // Build request
            let mut request = serde_json::json!({
                "model": self.model,
                "messages": messages,
            });

            if !self.tools.is_empty() {
                request["tools"] = serde_json::Value::Array(tool_defs.clone());
                request["tool_choice"] = serde_json::json!("auto");
            }

            if let Some(temp) = self.temperature {
                request["temperature"] = serde_json::json!(temp);
            }

            // Send request
            let response = self.send_request(&request).await?;

            // Extract message from response
            let message = response
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .ok_or_else(|| OpenAIError::Parse("No message in response".into()))?;

            // Check for tool calls
            let tool_calls = message
                .get("tool_calls")
                .and_then(|tc| tc.as_array())
                .cloned()
                .unwrap_or_default();

            if tool_calls.is_empty() {
                // No tool calls - we have a final response
                let content = message
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                info!(
                    iterations = iterations,
                    tool_calls_total = tool_calls_made.len(),
                    response_len = content.len(),
                    "Agent finished - final response received"
                );

                debug!(response_content = %content, "Agent final response content");

                return Ok(AgentResponse {
                    content,
                    tool_calls_made,
                    iterations,
                });
            }

            info!(
                iteration = iterations,
                tool_call_count = tool_calls.len(),
                "Agent received tool call request"
            );

            // Add assistant message with tool calls to history
            messages.push(message.clone());

            // Execute each tool call
            for tc_value in &tool_calls {
                let Some(tc) = ToolCall::from_openai_value(tc_value) else {
                    warn!("Failed to parse tool call: {:?}", tc_value);
                    continue;
                };

                info!(
                    tool = %tc.name,
                    id = %tc.id,
                    arguments = %tc.arguments,
                    "Executing tool call"
                );
                tool_calls_made.push(tc.name.clone());

                // Find and execute the tool
                let result = self.execute_tool(&tc).await;

                info!(
                    tool = %tc.name,
                    result_len = result.len(),
                    result_preview = %truncate_for_log(&result, 200),
                    "Tool execution complete"
                );

                // Add tool result to messages
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": tc.id,
                    "content": result
                }));
            }
        }
    }

    /// Execute a single tool call.
    async fn execute_tool(&self, call: &ToolCall) -> String {
        // Find the tool
        let tool = self.tools.iter().find(|t| t.name() == call.name);

        let Some(tool) = tool else {
            warn!(tool = %call.name, "Unknown tool requested");
            return format!("Error: Unknown tool '{}'", call.name);
        };

        // Execute
        match tool.call_erased(&call.arguments).await {
            Ok(result) => result,
            Err(e) => {
                warn!(tool = %call.name, error = %e, "Tool execution failed");
                format!("Error executing tool: {}", e)
            }
        }
    }

    /// Send a raw request to the OpenAI API.
    async fn send_request(&self, request: &serde_json::Value) -> Result<serde_json::Value> {
        debug!(
            model = %self.model,
            "Sending request to OpenAI API"
        );
        let response = reqwest::Client::new()
            .post(format!("{}/chat/completions", self.client.base_url()))
            .header("Authorization", format!("Bearer {}", self.client.api_key()))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenAIError::Api(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenAIError::Parse(e.to_string()))
    }
}

/// Truncate a string for logging purposes.
fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}...[truncated {} chars]",
            &s[..max_len],
            s.len() - max_len
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::Tool;
    use async_trait::async_trait;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, JsonSchema)]
    struct AddArgs {
        a: i32,
        b: i32,
    }

    #[derive(Serialize)]
    struct AddResult {
        sum: i32,
    }

    struct Calculator;

    #[async_trait]
    impl Tool for Calculator {
        const NAME: &'static str = "add";
        type Args = AddArgs;
        type Output = AddResult;
        type Error = std::convert::Infallible;

        fn description(&self) -> &str {
            "Add two numbers together"
        }

        async fn call(&self, args: Self::Args) -> std::result::Result<Self::Output, Self::Error> {
            Ok(AddResult {
                sum: args.a + args.b,
            })
        }
    }

    #[test]
    fn test_agent_builder() {
        let client = OpenAIClient::new("test-key");
        let _agent = client
            .agent("gpt-4o")
            .system("You are a helpful assistant")
            .tool(Calculator)
            .max_iterations(5)
            .temperature(0.7)
            .build();
    }

    #[test]
    fn test_tool_definitions() {
        let client = OpenAIClient::new("test-key");
        let agent = client.agent("gpt-4o").tool(Calculator).build();

        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.tools[0].name(), "add");
    }
}
