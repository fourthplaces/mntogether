use anyhow::Result;
use async_trait::async_trait;
use pgvector::Vector;
use sqlx::PgPool;

/// Trait for entities that support semantic embedding and similarity search
#[async_trait]
pub trait Embeddable: Sized {
    /// The ID type for this entity (e.g., Uuid, PostId)
    type Id: Send + Sync + for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>;

    /// The table name in the database (e.g., "members", "posts")
    fn table_name() -> &'static str;

    /// Update the embedding vector for this entity
    async fn update_embedding(id: Self::Id, embedding: &[f32], pool: &PgPool) -> Result<()> {
        let vector = Vector::from(embedding.to_vec());
        let query = format!(
            "UPDATE {} SET embedding = $2 WHERE id = $1",
            Self::table_name()
        );

        sqlx::query(&query)
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Search for entities by semantic similarity (optional, implement if needed)
    ///
    /// Returns a list of (entity, similarity_score) tuples where similarity_score
    /// is in the range [0, 1], with 1 being most similar.
    ///
    /// # Parameters
    /// - `query_embedding`: The embedding vector to compare against
    /// - `match_threshold`: Minimum similarity score (0-1) to include in results
    /// - `limit`: Maximum number of results to return
    ///
    /// Note: This is optional. Implement this if you need similarity search.
    async fn search_by_similarity(
        _query_embedding: &[f32],
        _match_threshold: f32,
        _limit: i32,
        _pool: &PgPool,
    ) -> Result<Vec<(Self, f32)>>
    where
        Self: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
    {
        // Default implementation returns empty results
        // Override this method in your implementation for actual search functionality
        Ok(Vec::new())
    }
}
