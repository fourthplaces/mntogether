//! OpenAI implementation of the AI trait.
//!
//! A reference implementation using OpenAI's GPT-4o and text-embedding-3-small.
//!
//! # Example
//!
//! ```rust,ignore
//! use extraction::ai::OpenAI;
//!
//! let ai = OpenAI::new("sk-...").with_model("gpt-4o");
//! let index = Index::new(store, ai);
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Truncate a string to at most `max_bytes` bytes, ensuring we don't cut in the middle
/// of a multi-byte UTF-8 character.
fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the last valid char boundary at or before max_bytes
    let mut end = max_bytes;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

/// Strip markdown code blocks from a response.
fn strip_code_blocks(response: &str) -> &str {
    response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
}

use crate::error::{ExtractionError, Result};
use crate::traits::ai::{ExtractionStrategy, Partition, AI};
use crate::types::{
    extraction::{Extraction, GapQuery, Source, SourceRole},
    page::CachedPage,
    summary::{RecallSignals, Summary, SummaryResponse},
};

/// OpenAI-based AI implementation.
///
/// Uses GPT-4-turbo for text generation and text-embedding-3-small for embeddings.
#[derive(Clone)]
pub struct OpenAI {
    client: Client,
    api_key: String,
    model: String,
    embedding_model: String,
    base_url: String,
}

impl OpenAI {
    /// Create a new OpenAI client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: "gpt-4o".to_string(), // gpt-4o supports structured outputs (json_schema)
            embedding_model: "text-embedding-3-small".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ExtractionError::Config("OPENAI_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    /// Set the chat model (default: gpt-4o).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the embedding model (default: text-embedding-3-small).
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Set a custom base URL (for Azure, proxies, etc.).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Get the current model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the API key (for bridge implementations that need it).
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    // =========================================================================
    // Generic AI methods (for server integration)
    // =========================================================================

    /// Generic chat completion (for server's BaseAI trait).
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        self.chat("You are a helpful assistant.", prompt).await
    }

    /// Chat completion with specific model override.
    pub async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let model_to_use = model.unwrap_or(&self.model);
        self.chat_with_model("You are a helpful assistant.", prompt, model_to_use)
            .await
    }

    /// Structured output with JSON schema (OpenAI's json_schema response_format).
    pub async fn generate_structured(
        &self,
        system: &str,
        user: &str,
        schema: serde_json::Value,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct StructuredRequest {
            model: String,
            messages: Vec<ChatMessage>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            response_format: ResponseFormat,
        }

        #[derive(Serialize)]
        struct ResponseFormat {
            #[serde(rename = "type")]
            format_type: String,
            json_schema: JsonSchemaFormat,
        }

        #[derive(Serialize)]
        struct JsonSchemaFormat {
            name: String,
            strict: bool,
            schema: serde_json::Value,
        }

        // Only set temperature for models that support it (not o1, o3, gpt-5, etc.)
        let temperature = if self.model.starts_with("o1")
            || self.model.starts_with("o3")
            || self.model.starts_with("gpt-5")
        {
            None // These models only support default temperature (1.0)
        } else {
            Some(0.0)
        };

        let request = StructuredRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
            temperature,
            response_format: ResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: JsonSchemaFormat {
                    name: "structured_response".to_string(),
                    strict: true,
                    schema,
                },
            },
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ExtractionError::AI(
                format!("OpenAI structured output error: {}", error_text).into(),
            ));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| ExtractionError::AI("No response from OpenAI".into()))
    }

    /// Tool calling support (for agentic extraction).
    pub async fn generate_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto"
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ExtractionError::AI(
                format!("OpenAI tools API error: {}", error_text).into(),
            ));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        // Extract the assistant message
        Ok(response_json["choices"][0]["message"].clone())
    }

    // =========================================================================
    // Internal methods
    // =========================================================================

    /// Make a chat completion request.
    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        self.chat_with_model(system, user, &self.model).await
    }

    /// Check if a model requires max_completion_tokens instead of max_tokens.
    fn uses_max_completion_tokens(model: &str) -> bool {
        // Newer models (o1, o3, gpt-5, etc.) require max_completion_tokens
        model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("gpt-5")
            || model.contains("-o1")
            || model.contains("-o3")
    }

    /// Make a chat completion request with specific model.
    async fn chat_with_model(&self, system: &str, user: &str, model: &str) -> Result<String> {
        let (max_completion_tokens, max_tokens, temperature) =
            if Self::uses_max_completion_tokens(model) {
                // Newer models: use max_completion_tokens, no temperature for reasoning models
                (Some(4096), None, None)
            } else {
                // Older models: use max_tokens with temperature
                (None, Some(4096), Some(0.0))
            };

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
            temperature,
            max_completion_tokens,
            max_tokens,
        };

        let start = std::time::Instant::now();
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "OpenAI request failed");
                ExtractionError::AI(e.to_string().into())
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(status = %status, error = %error_text, "OpenAI API error");
            return Err(ExtractionError::AI(
                format!("OpenAI API error: {}", error_text).into(),
            ));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        let result = chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| ExtractionError::AI("No response from OpenAI".into()))?;

        debug!(model = %model, duration_ms = start.elapsed().as_millis(), "OpenAI call");

        Ok(result)
    }

    /// Parse JSON response with retry - if parsing fails, ask AI to fix it.
    async fn parse_json_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        response: &str,
        context: &str,
    ) -> Result<T> {
        let cleaned = strip_code_blocks(response);

        // First attempt
        match serde_json::from_str::<T>(cleaned) {
            Ok(parsed) => return Ok(parsed),
            Err(first_error) => {
                warn!(
                    error = %first_error,
                    context = %context,
                    "JSON parse failed, asking AI to fix"
                );

                // Ask AI to fix the JSON
                let fix_prompt = format!(
                    "The following JSON is invalid. Fix it and return ONLY valid JSON, no explanation:\n\nError: {}\n\nInvalid JSON:\n{}",
                    first_error,
                    cleaned
                );

                let fixed_response = self
                    .chat("You are a JSON fixer. Return only valid JSON.", &fix_prompt)
                    .await?;
                let fixed_cleaned = strip_code_blocks(&fixed_response);

                serde_json::from_str::<T>(fixed_cleaned).map_err(|e| {
                    ExtractionError::AI(
                        format!("Failed to parse {} after retry: {}", context, e).into(),
                    )
                })
            }
        }
    }

    /// Make an embedding request.
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: text.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "Embedding request failed");
                ExtractionError::AI(e.to_string().into())
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(error = %error_text, "OpenAI embedding error");
            return Err(ExtractionError::AI(
                format!("OpenAI embedding error: {}", error_text).into(),
            ));
        }

        let embed_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        embed_response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| ExtractionError::AI("No embedding from OpenAI".into()))
    }
}

#[async_trait]
impl AI for OpenAI {
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse> {
        let system = r#"You are an extraction assistant. Summarize the page content and extract recall signals.

Output JSON with this structure:
{
  "summary": "A 2-3 sentence summary of the page",
  "signals": {
    "calls_to_action": ["action phrases like 'sign up', 'contact us'"],
    "offers": ["what the page offers - services, products, programs"],
    "asks": ["what the page asks for - volunteers, donations, applications"],
    "entities": ["key entities - names, dates, contacts, locations"]
  }
}

Be factual. Only extract what's explicitly stated."#;

        let truncated_content = truncate_to_char_boundary(content, 12000);
        let user = format!("URL: {}\n\nContent:\n{}", url, truncated_content);

        let response = self.chat(system, &user).await?;

        // Parse JSON response with retry if needed
        let parsed: SummaryJsonResponse = self.parse_json_with_retry(&response, "summary").await?;

        Ok(SummaryResponse {
            summary: parsed.summary,
            signals: RecallSignals {
                calls_to_action: parsed.signals.calls_to_action,
                offers: parsed.signals.offers,
                asks: parsed.signals.asks,
                entities: parsed.signals.entities,
            },
            language: None,
        })
    }

    async fn expand_query(&self, query: &str) -> Result<Vec<String>> {
        let system = "Generate 5 related search terms for the query. Return as JSON array.";
        let response = self.chat(system, query).await?;

        // Try parsing with retry, fall back to original query if all parsing fails
        match self
            .parse_json_with_retry::<Vec<String>>(&response, "expand_query")
            .await
        {
            Ok(terms) => Ok(terms),
            Err(_) => Ok(vec![query.to_string()]),
        }
    }

    async fn classify_query(&self, query: &str) -> Result<ExtractionStrategy> {
        let system = r#"Classify the query intent. Return JSON:
{"strategy": "collection" | "singular" | "narrative", "reasoning": "..."}

- collection: "Find all X", lists, multiple items
- singular: Point lookup, specific fact, contact info
- narrative: Summarize, describe, overview"#;

        let response = self.chat(system, query).await?;

        #[derive(Deserialize)]
        struct Classification {
            strategy: String,
        }

        // Try parsing with retry, default to collection if all parsing fails
        let parsed: Classification = self
            .parse_json_with_retry(&response, "classify_query")
            .await
            .unwrap_or(Classification {
                strategy: "collection".to_string(),
            });

        Ok(match parsed.strategy.as_str() {
            "singular" => ExtractionStrategy::Singular,
            "narrative" => ExtractionStrategy::Narrative,
            _ => ExtractionStrategy::Collection,
        })
    }

    async fn recall_and_partition(
        &self,
        query: &str,
        summaries: &[Summary],
    ) -> Result<Vec<Partition>> {
        if summaries.is_empty() {
            return Ok(vec![]);
        }

        let system = r#"Given summaries, identify distinct items matching the query and group pages.

Output JSON:
{
  "partitions": [
    {"title": "Item Name", "urls": ["url1", "url2"], "rationale": "Why grouped"}
  ]
}

Each distinct item should be its own partition."#;

        let summaries_text: String = summaries
            .iter()
            .map(|s| format!("URL: {}\nSummary: {}\n", s.url, s.text))
            .collect::<Vec<_>>()
            .join("\n---\n");

        let user = format!("Query: {}\n\nSummaries:\n{}", query, summaries_text);
        let response = self.chat(system, &user).await?;

        #[derive(Deserialize)]
        struct PartitionResponse {
            partitions: Vec<PartitionItem>,
        }

        #[derive(Deserialize)]
        struct PartitionItem {
            title: String,
            urls: Vec<String>,
            rationale: String,
        }

        // Try parsing with retry, default to empty partitions if all parsing fails
        let parsed: PartitionResponse = self
            .parse_json_with_retry(&response, "recall_and_partition")
            .await
            .unwrap_or(PartitionResponse { partitions: vec![] });

        Ok(parsed
            .partitions
            .into_iter()
            .map(|p| Partition {
                title: p.title,
                urls: p.urls,
                rationale: p.rationale,
            })
            .collect())
    }

    async fn extract(
        &self,
        query: &str,
        pages: &[CachedPage],
        _hints: Option<&[String]>,
    ) -> Result<Extraction> {
        if pages.is_empty() {
            return Ok(Extraction::new("No pages to extract from.".to_string()));
        }

        let system = r#"Extract comprehensive information matching the query from the pages. Be thorough and evidence-grounded.

Output JSON:
{
  "content": "Comprehensive markdown extraction with all relevant details, descriptions, contact information, locations, schedules, and requirements. Use citations [1], [2] to attribute sources.",
  "sources_used": ["url1", "url2"],
  "gaps": [{"field": "missing field", "query": "search query to find it"}],
  "has_conflicts": false,
  "conflicts": []
}

Include all relevant details found in the sources. Be thorough - extract everything that could be useful. Mark anything inferred."#;

        let pages_text: String = pages
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "[{}] URL: {}\nTitle: {}\nContent:\n{}\n",
                    i + 1,
                    p.url,
                    p.title.as_deref().unwrap_or("Untitled"),
                    truncate_to_char_boundary(&p.content, 8000)
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n");

        let user = format!("Query: {}\n\nPages:\n{}", query, pages_text);
        let response = self.chat(system, &user).await?;

        #[derive(Deserialize)]
        struct ExtractionResponse {
            content: String,
            sources_used: Vec<String>,
            gaps: Vec<GapItem>,
            #[allow(dead_code)]
            has_conflicts: bool,
        }

        #[derive(Deserialize)]
        struct GapItem {
            field: String,
            query: String,
        }

        // Parse with retry - if parsing fails, ask AI to fix the JSON
        let parsed: ExtractionResponse =
            self.parse_json_with_retry(&response, "extraction").await?;

        let sources: Vec<Source> = parsed
            .sources_used
            .into_iter()
            .enumerate()
            .map(|(i, url)| {
                let page = pages.iter().find(|p| p.url == url);
                Source {
                    url,
                    title: page.and_then(|p| p.title.clone()),
                    fetched_at: page.map(|p| p.fetched_at).unwrap_or_else(chrono::Utc::now),
                    role: if i == 0 {
                        SourceRole::Primary
                    } else {
                        SourceRole::Supporting
                    },
                    metadata: std::collections::HashMap::new(),
                }
            })
            .collect();

        let grounding = Extraction::calculate_grounding(&sources, &[], false);

        let gaps: Vec<GapQuery> = parsed
            .gaps
            .into_iter()
            .map(|g| GapQuery::new(g.field, g.query))
            .collect();

        let status = if parsed.content.is_empty() && !gaps.is_empty() {
            crate::types::extraction::ExtractionStatus::Missing
        } else if !gaps.is_empty() {
            crate::types::extraction::ExtractionStatus::Partial
        } else {
            crate::types::extraction::ExtractionStatus::Found
        };

        info!(
            pages = pages.len(),
            content_len = parsed.content.len(),
            sources = sources.len(),
            gaps = gaps.len(),
            "Extraction complete"
        );

        Ok(Extraction {
            content: parsed.content,
            sources,
            gaps,
            grounding,
            conflicts: vec![],
            status,
        })
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_text(text).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed_text(text).await?);
        }
        Ok(results)
    }
}

// Request/Response types

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// For newer models (o1, o3, gpt-5), use max_completion_tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    /// For older models (gpt-4o, gpt-4, etc.), use max_tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct SummaryJsonResponse {
    summary: String,
    signals: SignalsJson,
}

#[derive(Deserialize)]
struct SignalsJson {
    #[serde(default)]
    calls_to_action: Vec<String>,
    #[serde(default)]
    offers: Vec<String>,
    #[serde(default)]
    asks: Vec<String>,
    #[serde(default)]
    entities: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_builder() {
        let ai = OpenAI::new("sk-test")
            .with_model("gpt-4o-mini")
            .with_embedding_model("text-embedding-3-large")
            .with_base_url("https://custom.api.com");

        assert_eq!(ai.model, "gpt-4o-mini");
        assert_eq!(ai.embedding_model, "text-embedding-3-large");
        assert_eq!(ai.base_url, "https://custom.api.com");
    }
}
