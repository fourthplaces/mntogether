//! Semantic search activities for posts

use anyhow::Result;

use crate::domains::posts::models::post::{Post, PostSearchResult};
use crate::kernel::ServerDeps;

/// Search posts using semantic similarity.
///
/// Generates an embedding for the query then searches by cosine similarity.
pub async fn search_posts_semantic(
    query: &str,
    threshold: f32,
    limit: i32,
    deps: &ServerDeps,
) -> Result<Vec<PostSearchResult>> {
    let query_embedding = deps.ai.create_embedding(query, "text-embedding-3-small").await?;

    Post::search_by_similarity(&query_embedding, threshold, limit, &deps.db_pool).await
}
