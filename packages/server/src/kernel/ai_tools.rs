//! Reusable AI tools for agentic workflows.
//!
//! These tools implement the `openai_client::Tool` trait and can be used
//! with the Agent builder for tool-calling loops.

use std::sync::Arc;

use async_trait::async_trait;
use extraction::{Ingestor, WebSearcher};
use openai_client::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;

use crate::domains::posts::models::post::Post;
use crate::kernel::traits::BaseEmbeddingService;

/// Error type for AI tools.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Web search failed: {0}")]
    WebSearch(String),

    #[error("Page fetch failed: {0}")]
    FetchPage(String),

    #[error("Post search failed: {0}")]
    PostSearch(String),
}

// =============================================================================
// Web Search Tool
// =============================================================================

/// Arguments for web search.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WebSearchArgs {
    /// The search query.
    pub query: String,
}

/// A single search result.
#[derive(Debug, Serialize)]
pub struct SearchResultOutput {
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
}

/// Tool for searching the web using Tavily.
pub struct WebSearchTool {
    searcher: Arc<dyn WebSearcher>,
}

impl WebSearchTool {
    pub fn new(searcher: Arc<dyn WebSearcher>) -> Self {
        Self { searcher }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";
    type Args = WebSearchArgs;
    type Output = Vec<SearchResultOutput>;
    type Error = ToolError;

    fn description(&self) -> &str {
        "Search the web for information. Use this to find contact info, addresses, hours, or other details about an organization."
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let results = self
            .searcher
            .search_with_limit(&args.query, 5)
            .await
            .map_err(|e| ToolError::WebSearch(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| SearchResultOutput {
                url: r.url.to_string(),
                title: r.title,
                snippet: r.snippet,
            })
            .collect())
    }
}

// =============================================================================
// Fetch Page Tool
// =============================================================================

/// Arguments for fetching a page.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FetchPageArgs {
    /// The URL to fetch.
    pub url: String,
}

/// Output from fetching a page.
#[derive(Debug, Serialize)]
pub struct FetchPageOutput {
    pub url: String,
    pub content: String,
    pub title: Option<String>,
}

/// Tool for fetching and extracting content from a URL.
pub struct FetchPageTool {
    ingestor: Arc<dyn Ingestor>,
}

impl FetchPageTool {
    pub fn new(ingestor: Arc<dyn Ingestor>) -> Self {
        Self { ingestor }
    }
}

#[async_trait]
impl Tool for FetchPageTool {
    const NAME: &'static str = "fetch_page";
    type Args = FetchPageArgs;
    type Output = FetchPageOutput;
    type Error = ToolError;

    fn description(&self) -> &str {
        "Fetch the content of a web page. Use this to get detailed information from a specific URL like a contact page or about page."
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let page = self
            .ingestor
            .fetch_one(&args.url)
            .await
            .map_err(|e| ToolError::FetchPage(e.to_string()))?;

        // Truncate content to avoid overwhelming the model
        let content = if page.content.len() > 8000 {
            format!("{}...\n\n[Content truncated]", &page.content[..8000])
        } else {
            page.content
        };

        Ok(FetchPageOutput {
            url: args.url,
            content,
            title: page.title,
        })
    }
}

// =============================================================================
// Search Posts Tool (semantic vector search)
// =============================================================================

/// Arguments for searching posts.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPostsArgs {
    /// Natural language search query describing what to look for.
    /// Examples: "food shelves near downtown", "volunteer opportunities", "legal aid services"
    pub query: String,
}

/// A single post search result.
#[derive(Debug, Serialize)]
pub struct SearchPostOutput {
    pub post_id: String,
    pub title: String,
    pub description: String,
    pub tldr: Option<String>,
    pub category: String,
    pub post_type: String,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub similarity: f64,
}

/// Tool for searching posts using semantic vector similarity.
pub struct SearchPostsTool {
    db_pool: PgPool,
    embedding_service: Arc<dyn BaseEmbeddingService>,
}

impl SearchPostsTool {
    pub fn new(db_pool: PgPool, embedding_service: Arc<dyn BaseEmbeddingService>) -> Self {
        Self {
            db_pool,
            embedding_service,
        }
    }
}

#[async_trait]
impl Tool for SearchPostsTool {
    const NAME: &'static str = "search_posts";
    type Args = SearchPostsArgs;
    type Output = Vec<SearchPostOutput>;
    type Error = ToolError;

    fn description(&self) -> &str {
        "Search for services, opportunities, businesses, and resources. Use this to find posts matching a user's question about what's available in their community."
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let embedding = self
            .embedding_service
            .generate(&args.query)
            .await
            .map_err(|e| ToolError::PostSearch(format!("Embedding generation failed: {}", e)))?;

        let results = Post::search_by_similarity_with_location(&embedding, 0.3, 10, &self.db_pool)
            .await
            .map_err(|e| ToolError::PostSearch(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| SearchPostOutput {
                post_id: r.post_id.to_string(),
                title: r.title,
                description: r.description,
                tldr: r.tldr,
                category: r.category,
                post_type: r.post_type,
                location: r.location,
                source_url: r.source_url,
                similarity: r.similarity,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_args_schema() {
        // Verify the schema can be generated
        let schema = schemars::schema_for!(WebSearchArgs);
        assert!(schema.schema.object.is_some());
    }

    #[test]
    fn test_fetch_page_args_schema() {
        let schema = schemars::schema_for!(FetchPageArgs);
        assert!(schema.schema.object.is_some());
    }

    #[test]
    fn test_search_posts_args_schema() {
        let schema = schemars::schema_for!(SearchPostsArgs);
        assert!(schema.schema.object.is_some());
    }
}
