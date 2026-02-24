//! Generate embedding action - creates embedding vector for member's searchable text
//!
//! This is a pure action that returns a result - event emission happens in the effect handler.

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{debug, info};
use uuid::Uuid;

use crate::domains::member::models::member::Member;
use crate::kernel::BaseEmbeddingService;

/// Result of embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub member_id: Uuid,
    pub dimensions: usize,
}

/// Generate embedding for a member's searchable text.
///
/// This is a pure action - it returns a Result, not events.
/// The effect handler wraps this and emits events.
///
/// Returns:
/// - `Ok(EmbeddingResult)` on success
/// - `Err` if member not found or generation fails
pub async fn generate_embedding(
    member_id: Uuid,
    embedding_service: &dyn BaseEmbeddingService,
    pool: &PgPool,
) -> Result<EmbeddingResult> {
    info!("Generating embedding for member: {}", member_id);

    // Get member from database
    let member = Member::find_by_id(member_id, pool)
        .await
        .context(format!("Member not found: {}", member_id))?;

    // Generate embedding using the embedding service
    let embedding = embedding_service
        .generate(&member.searchable_text)
        .await
        .context("Embedding generation failed")?;

    debug!("Generated embedding with {} dimensions", embedding.len());

    // Update member with embedding
    Member::update_embedding(member_id, &embedding, pool)
        .await
        .context("Failed to save embedding")?;

    info!("Embedding generated and saved for member: {}", member_id);

    Ok(EmbeddingResult {
        member_id,
        dimensions: embedding.len(),
    })
}
