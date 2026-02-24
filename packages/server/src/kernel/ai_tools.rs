//! Reusable AI tools for agentic workflows.
//!
//! These tools implement the `ai_client::Tool` trait and can be used
//! with the Agent builder for tool-calling loops.

use std::sync::Arc;

use ai_client::{Tool, ToolDefinition};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;

use crate::domains::posts::models::post::Post;
use crate::kernel::traits::BaseEmbeddingService;

/// Error type for AI tools.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Post search failed: {0}")]
    PostSearch(String),
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
    pub summary: Option<String>,
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

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search for services, opportunities, businesses, and resources. Use this to find posts matching a user's question about what's available in their community.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SearchPostsArgs)).unwrap_or_default(),
        }
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
                summary: r.summary,
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
    fn test_search_posts_args_schema() {
        let schema = schemars::schema_for!(SearchPostsArgs);
        assert!(schema.schema.object.is_some());
    }
}
