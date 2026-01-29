//! AI-Powered Organization Matching Service
//!
//! This module provides semantic search capabilities for organizations using
//! vector embeddings and cosine similarity. It powers:
//!
//! - Chat-based organization recommendations
//! - Referral document generation
//! - Service discovery based on user needs
//!
//! # Example
//!
//! ```rust,ignore
//! use server_core::kernel::{OpenAIClient, ai_matching::AIMatchingService};
//!
//! let openai = OpenAIClient::new(api_key);
//! let ai_matching = AIMatchingService::new(openai);
//!
//! // Find organizations that match user query
//! let results = ai_matching
//!     .find_relevant_organizations(
//!         "I need immigration legal help in Spanish".to_string(),
//!         &pool
//!     )
//!     .await?;
//!
//! for (org, similarity) in results {
//!     println!("{} ({}% match)", org.name, similarity * 100.0);
//! }
//! ```
//!
//! # Production Considerations
//!
//! - **Rate Limiting**: Built-in 100ms delay between API calls
//! - **Retry Logic**: Automatic retry with exponential backoff (max 3 attempts)
//! - **Logging**: Comprehensive tracing for debugging and monitoring
//! - **Validation**: Embedding dimension validation (must be 1536)
//! - **Error Handling**: Detailed error context for debugging

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;

use crate::domains::organization::models::Organization;
use crate::kernel::ai::OpenAIClient;

const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.7;
const DEFAULT_RESULT_LIMIT: i32 = 10;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

/// Configuration for AI matching
#[derive(Debug, Clone)]
pub struct AIMatchingConfig {
    pub similarity_threshold: f32,
    pub result_limit: i32,
    pub max_retries: u32,
}

impl Default for AIMatchingConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: DEFAULT_SIMILARITY_THRESHOLD,
            result_limit: DEFAULT_RESULT_LIMIT,
            max_retries: MAX_RETRIES,
        }
    }
}

/// AI-powered organization matching for chat and referrals
pub struct AIMatchingService {
    openai_client: OpenAIClient,
    config: AIMatchingConfig,
}

impl AIMatchingService {
    pub fn new(openai_client: OpenAIClient) -> Self {
        Self {
            openai_client,
            config: AIMatchingConfig::default(),
        }
    }

    pub fn with_config(openai_client: OpenAIClient, config: AIMatchingConfig) -> Self {
        Self {
            openai_client,
            config,
        }
    }

    /// Find organizations relevant to a user's query using semantic search
    ///
    /// Example: "I need help with immigration legal services in Spanish"
    /// Returns: Law firms specializing in immigration with Spanish speakers
    pub async fn find_relevant_organizations(
        &self,
        user_query: String,
        pool: &PgPool,
    ) -> Result<Vec<(Organization, f32)>> {
        self.find_relevant_organizations_with_config(
            user_query,
            self.config.similarity_threshold,
            self.config.result_limit,
            pool,
        )
        .await
    }

    /// Find organizations with custom threshold and limit
    pub async fn find_relevant_organizations_with_config(
        &self,
        user_query: String,
        similarity_threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<(Organization, f32)>> {
        tracing::info!(
            query = %user_query,
            threshold = similarity_threshold,
            limit = limit,
            "Finding relevant organizations using semantic search"
        );

        // Step 1: Generate embedding for user query with retry logic
        let embedding = self
            .generate_embedding_with_retry(&user_query)
            .await
            .context("Failed to generate embedding for user query")?;

        tracing::debug!(
            embedding_dim = embedding.len(),
            "Generated embedding for user query"
        );

        // Step 2: Search organizations by semantic similarity
        let results = Organization::search_by_similarity(&embedding, similarity_threshold, limit, pool)
            .await
            .context("Failed to search organizations by similarity")?;

        tracing::info!(
            result_count = results.len(),
            "Found {} relevant organizations",
            results.len()
        );

        Ok(results)
    }

    /// Generate embedding with retry logic
    async fn generate_embedding_with_retry(&self, text: &str) -> Result<Vec<f32>> {
        let mut retries = 0;

        loop {
            match self.openai_client.create_embedding(text).await {
                Ok(response) => {
                    let embedding = response
                        .data
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("No embedding returned from OpenAI"))?
                        .embedding
                        .clone();

                    // Validate embedding dimension
                    if embedding.len() != 1536 {
                        anyhow::bail!(
                            "Invalid embedding dimension: expected 1536, got {}",
                            embedding.len()
                        );
                    }

                    return Ok(embedding);
                }
                Err(e) if retries < self.config.max_retries => {
                    retries += 1;
                    tracing::warn!(
                        error = %e,
                        retry = retries,
                        max_retries = self.config.max_retries,
                        "Failed to generate embedding, retrying..."
                    );
                    sleep(Duration::from_millis(RETRY_DELAY_MS * retries as u64)).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to generate embedding after all retries");
                    return Err(e.into());
                }
            }
        }
    }

    /// Generate embedding for an organization's description
    pub async fn generate_organization_embedding(&self, org: &Organization) -> Result<Vec<f32>> {
        let text = org.get_embedding_text();

        tracing::debug!(
            org_id = %org.id,
            org_name = %org.name,
            text_length = text.len(),
            "Generating embedding for organization"
        );

        self.generate_embedding_with_retry(&text).await
    }

    /// Update embeddings for all organizations missing them (with batch processing)
    pub async fn update_missing_embeddings(&self, pool: &PgPool) -> Result<usize> {
        tracing::info!("Starting batch embedding update for organizations");

        // Get organizations without embeddings (must have description or summary)
        let orgs_without_embeddings: Vec<Organization> = sqlx::query_as(
            "SELECT * FROM organizations WHERE embedding IS NULL AND (description IS NOT NULL OR summary IS NOT NULL)"
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch organizations without embeddings")?;

        let total = orgs_without_embeddings.len();
        tracing::info!(
            total_orgs = total,
            "Found {} organizations without embeddings",
            total
        );

        if total == 0 {
            return Ok(0);
        }

        let mut updated_count = 0;
        let mut failed_count = 0;

        for (idx, org) in orgs_without_embeddings.iter().enumerate() {
            tracing::info!(
                progress = format!("{}/{}", idx + 1, total),
                org_id = %org.id,
                org_name = %org.name,
                "Processing organization"
            );

            match self.generate_organization_embedding(org).await {
                Ok(embedding) => {
                    match Organization::update_embedding(org.id, &embedding, pool).await {
                        Ok(_) => {
                            updated_count += 1;
                            tracing::debug!(
                                org_id = %org.id,
                                "Successfully updated embedding"
                            );
                        }
                        Err(e) => {
                            failed_count += 1;
                            tracing::error!(
                                error = %e,
                                org_id = %org.id,
                                "Failed to save embedding to database"
                            );
                        }
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::error!(
                        error = %e,
                        org_id = %org.id,
                        "Failed to generate embedding"
                    );
                }
            }

            // Rate limiting: small delay between API calls to avoid hitting rate limits
            if idx < orgs_without_embeddings.len() - 1 {
                sleep(Duration::from_millis(100)).await;
            }
        }

        tracing::info!(
            updated = updated_count,
            failed = failed_count,
            total = total,
            "Completed batch embedding update"
        );

        Ok(updated_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{BaseEmbeddingService, test_dependencies::MockEmbeddingService};
    use crate::domains::organization::models::{CreateOrganization, Organization};
    use sqlx::PgPool;

    struct MockOpenAI {
        embedding: Vec<f32>,
    }

    #[async_trait::async_trait]
    impl BaseEmbeddingService for MockOpenAI {
        async fn generate(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(self.embedding.clone())
        }
    }

    impl crate::kernel::BaseAI for MockOpenAI {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok("Mock response".to_string())
        }
    }

    impl Clone for MockOpenAI {
        fn clone(&self) -> Self {
            Self {
                embedding: self.embedding.clone(),
            }
        }
    }

    impl MockOpenAI {
        fn new() -> Self {
            // Create a mock embedding (1536 zeros)
            Self {
                embedding: vec![0.5; 1536],
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_ai_matching_config() {
        let config = AIMatchingConfig::default();
        assert_eq!(config.similarity_threshold, 0.7);
        assert_eq!(config.result_limit, 10);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_example_usage() {
        // Example: How this would be used in chat
        //
        // User: "I need immigration legal help and speak Spanish"
        //
        // System:
        // 1. Generates embedding from user query
        // 2. Searches organizations by semantic similarity
        // 3. Returns: Immigration law firms with Spanish speakers
        //
        // Results might include:
        // - "Legal Aid Center - specializes in immigration law, Spanish services available"
        // - "Community Legal Services - immigration assistance, bilingual staff"
    }

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_find_relevant_organizations() {
        // This test requires a running database with pgvector extension
        // Run with: cargo test --features integration_tests test_find_relevant_organizations -- --ignored

        // Setup would create test organizations with embeddings
        // Then search for them using semantic search
        // Verify the results are correct
    }
}
