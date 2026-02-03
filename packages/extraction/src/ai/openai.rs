//! OpenAI implementation of the AI trait.
//!
//! A reference implementation using OpenAI's GPT-4 and text-embedding-3-small.
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

use crate::error::{ExtractionError, Result};
use crate::traits::ai::{ExtractionStrategy, Partition, AI};
use crate::types::{
    extraction::{Extraction, GapQuery, Source, SourceRole},
    page::CachedPage,
    summary::{RecallSignals, Summary, SummaryResponse},
};

/// OpenAI-based AI implementation.
///
/// Uses GPT-4o for text generation and text-embedding-3-small for embeddings.
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
            model: "gpt-4o".to_string(),
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
            temperature: f32,
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
            temperature: 0.0,
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

    /// Make a chat completion request with specific model.
    async fn chat_with_model(&self, system: &str, user: &str, model: &str) -> Result<String> {
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
            temperature: Some(0.0),
            max_tokens: Some(4096),
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
                format!("OpenAI API error: {}", error_text).into(),
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
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
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

        let user = format!(
            "URL: {}\n\nContent:\n{}",
            url,
            &content[..content.len().min(12000)]
        );

        let response = self.chat(system, &user).await?;

        // Parse JSON response
        let parsed: SummaryJsonResponse = serde_json::from_str(&response)
            .or_else(|_| {
                // Try to extract JSON from markdown code block
                let json_str = response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(json_str)
            })
            .map_err(|e| ExtractionError::AI(format!("Failed to parse summary: {}", e).into()))?;

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

        let terms: Vec<String> = serde_json::from_str(&response)
            .or_else(|_| {
                let json_str = response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(json_str)
            })
            .unwrap_or_else(|_| vec![query.to_string()]);

        Ok(terms)
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

        let parsed: Classification = serde_json::from_str(&response)
            .or_else(|_| {
                let json_str = response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(json_str)
            })
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

        let parsed: PartitionResponse = serde_json::from_str(&response)
            .or_else(|_| {
                let json_str = response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(json_str)
            })
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

        let system = r#"Extract information matching the query from the pages. Be evidence-grounded.

Output JSON:
{
  "content": "Markdown formatted extraction with citations [1], [2]",
  "sources_used": ["url1", "url2"],
  "gaps": [{"field": "missing field", "query": "search query to find it"}],
  "has_conflicts": false,
  "conflicts": []
}

Only include information explicitly stated in the sources. Mark anything inferred."#;

        let pages_text: String = pages
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "[{}] URL: {}\nTitle: {}\nContent:\n{}\n",
                    i + 1,
                    p.url,
                    p.title.as_deref().unwrap_or("Untitled"),
                    &p.content[..p.content.len().min(8000)]
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
            has_conflicts: bool,
        }

        #[derive(Deserialize)]
        struct GapItem {
            field: String,
            query: String,
        }

        let parsed: ExtractionResponse = serde_json::from_str(&response)
            .or_else(|_| {
                let json_str = response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(json_str)
            })
            .map_err(|e| {
                ExtractionError::AI(format!("Failed to parse extraction: {}", e).into())
            })?;

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
        // OpenAI supports batch embeddings, but for simplicity use sequential
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
    temperature: Option<f32>,
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
