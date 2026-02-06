//! OpenAI implementation of the AI trait.
//!
//! Uses the `openai-client` crate for API communication and implements
//! extraction-specific logic (summarization, classification, etc.).
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
use openai_client::{ChatRequest, Message, OpenAIClient};
use serde::Deserialize;
use tracing::{info, warn};

use crate::error::{ExtractionError, Result};
use crate::traits::ai::{ExtractionStrategy, Partition, AI};
use crate::types::{
    extraction::{Extraction, GapQuery, Source, SourceRole},
    page::CachedPage,
    summary::{RecallSignals, Summary, SummaryResponse},
};

/// OpenAI-based AI implementation for extraction.
///
/// Wraps the pure `openai-client` and implements extraction-specific logic.
#[derive(Clone)]
pub struct OpenAI {
    client: OpenAIClient,
    model: String,
    embedding_model: String,
}

impl OpenAI {
    /// Create a new OpenAI extraction AI with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: OpenAIClient::new(api_key),
            model: "gpt-4o".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
        }
    }

    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let client =
            OpenAIClient::from_env().map_err(|e| ExtractionError::Config(e.to_string().into()))?;
        Ok(Self {
            client,
            model: "gpt-4o".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
        })
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
        self.client = self.client.with_base_url(url);
        self
    }

    /// Get the current model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the API key (for bridge implementations that need it).
    pub fn api_key(&self) -> &str {
        self.client.api_key()
    }

    // =========================================================================
    // Generic AI methods (for server integration)
    // =========================================================================

    /// Generic chat completion (for server's BaseAI trait).
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        let request = ChatRequest::new(&self.model)
            .message(Message::system("You are a helpful assistant."))
            .message(Message::user(prompt));

        let response = self
            .client
            .chat_completion(request)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        Ok(response.content)
    }

    /// Chat completion with specific model override.
    pub async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let model_to_use = model.unwrap_or(&self.model);

        let request = ChatRequest::new(model_to_use)
            .message(Message::system("You are a helpful assistant."))
            .message(Message::user(prompt));

        let response = self
            .client
            .chat_completion(request)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        Ok(response.content)
    }

    /// Structured output with JSON schema (OpenAI's json_schema response_format).
    pub async fn generate_structured(
        &self,
        system: &str,
        user: &str,
        schema: serde_json::Value,
    ) -> Result<String> {
        let request = openai_client::StructuredRequest::new(&self.model, system, user, schema);

        self.client
            .structured_output(request)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    /// Tool calling support (for agentic extraction).
    pub async fn generate_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request =
            openai_client::FunctionRequest::new(&self.model, messages.to_vec(), tools.clone());

        let response = self
            .client
            .function_calling(request)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        Ok(response.message)
    }

    // =========================================================================
    // Internal helper methods
    // =========================================================================

    /// Make a chat request and get response.
    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        let mut request = ChatRequest::new(&self.model)
            .message(Message::system(system))
            .message(Message::user(user));

        // Configure tokens based on model type
        if ChatRequest::uses_max_completion_tokens(&self.model) {
            request = request.max_completion_tokens(4096);
        } else {
            request = request.max_tokens(4096).temperature(0.0);
        }

        let response = self
            .client
            .chat_completion(request)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))?;

        Ok(response.content)
    }

    /// Parse JSON response with retry - if parsing fails, ask AI to fix it.
    async fn parse_json_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        response: &str,
        context: &str,
    ) -> Result<T> {
        let cleaned = openai_client::strip_code_blocks(response);

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
                let fixed_cleaned = openai_client::strip_code_blocks(&fixed_response);

                serde_json::from_str::<T>(fixed_cleaned).map_err(|e| {
                    ExtractionError::AI(
                        format!("Failed to parse {} after retry: {}", context, e).into(),
                    )
                })
            }
        }
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

        let truncated_content = openai_client::truncate_to_char_boundary(content, 12000);
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
                    openai_client::truncate_to_char_boundary(&p.content, 8000)
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
        self.client
            .create_embedding(text, &self.embedding_model)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.client
            .create_embeddings_batch(texts, &self.embedding_model)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }
}

// Extraction-specific response types

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
    }
}
