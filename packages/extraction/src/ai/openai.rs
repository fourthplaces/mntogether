//! OpenAI implementation of the AI trait.
//!
//! Uses the `ai-client` crate for API communication and implements
//! extraction-specific logic (summarization, classification, etc.).

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{info, warn};

use ai_client::OpenAi;

use crate::error::{ExtractionError, Result};
use crate::traits::ai::{ExtractionStrategy, Partition, AI};
use crate::types::{
    extraction::{Extraction, GapQuery, Source, SourceRole},
    page::CachedPage,
    summary::{RecallSignals, Summary, SummaryResponse},
};

/// OpenAI-based AI implementation for extraction.
///
/// Wraps the `ai-client` OpenAi struct and implements extraction-specific logic.
#[derive(Clone)]
pub struct OpenAI {
    client: OpenAi,
}

impl OpenAI {
    /// Create a new OpenAI extraction AI with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: OpenAi::new(api_key, "gpt-4o"),
        }
    }

    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let client =
            OpenAi::from_env("gpt-4o").map_err(|e| ExtractionError::Config(e.to_string().into()))?;
        Ok(Self { client })
    }

    /// Set the chat model (default: gpt-4o).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.client = OpenAi::new(self.client.api_key(), model);
        if let Some(url) = self.base_url() {
            self.client = self.client.with_base_url(url);
        }
        self
    }

    /// Set the embedding model (default: text-embedding-3-small).
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.client = self.client.with_embedding_model(model);
        self
    }

    /// Set a custom base URL (for Azure, proxies, etc.).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.client = self.client.with_base_url(url);
        self
    }

    /// Get the current model name.
    pub fn model(&self) -> &str {
        self.client.model()
    }

    /// Get the API key (for bridge implementations that need it).
    pub fn api_key(&self) -> &str {
        self.client.api_key()
    }

    fn base_url(&self) -> Option<String> {
        // OpenAi doesn't expose base_url, so we track it separately if needed
        None
    }

    // =========================================================================
    // Generic AI methods (for server integration)
    // =========================================================================

    /// Generic chat completion (for server's BaseAI trait).
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        self.client
            .complete(prompt)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    /// Chat completion with specific model override.
    pub async fn complete_with_model(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        if let Some(model) = model {
            let temp_client = OpenAi::new(self.client.api_key(), model);
            temp_client
                .complete(prompt)
                .await
                .map_err(|e| ExtractionError::AI(e.to_string().into()))
        } else {
            self.complete(prompt).await
        }
    }

    /// Structured output with JSON schema.
    pub async fn generate_structured(
        &self,
        system: &str,
        user: &str,
        schema: serde_json::Value,
    ) -> Result<String> {
        self.client
            .structured_output(system, user, schema)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    /// Tool calling support (for agentic extraction).
    pub async fn generate_with_tools(
        &self,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.client
            .function_calling(messages, tools)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    // =========================================================================
    // Internal helper methods
    // =========================================================================

    /// Make a chat request and get response.
    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        self.client
            .chat_completion(system, user)
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    /// Parse JSON response with retry - if parsing fails, ask AI to fix it.
    async fn parse_json_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        response: &str,
        context: &str,
    ) -> Result<T> {
        let cleaned = ai_client::strip_code_blocks(response);

        match serde_json::from_str::<T>(cleaned) {
            Ok(parsed) => return Ok(parsed),
            Err(first_error) => {
                warn!(
                    error = %first_error,
                    context = %context,
                    "JSON parse failed, asking AI to fix"
                );

                let fix_prompt = format!(
                    "The following JSON is invalid. Fix it and return ONLY valid JSON, no explanation:\n\nError: {}\n\nInvalid JSON:\n{}",
                    first_error,
                    cleaned
                );

                let fixed_response = self
                    .chat("You are a JSON fixer. Return only valid JSON.", &fix_prompt)
                    .await?;
                let fixed_cleaned = ai_client::strip_code_blocks(&fixed_response);

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

        let truncated_content = ai_client::truncate_to_char_boundary(content, 12000);
        let user = format!("URL: {}\n\nContent:\n{}", url, truncated_content);

        let response = self.chat(system, &user).await?;

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
                    ai_client::truncate_to_char_boundary(&p.content, 8000)
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
            .create_embedding(text, "text-embedding-3-small")
            .await
            .map_err(|e| ExtractionError::AI(e.to_string().into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.client
            .create_embeddings_batch(texts, "text-embedding-3-small")
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
            .with_embedding_model("text-embedding-3-large");

        assert_eq!(ai.model(), "gpt-4o-mini");
    }
}
