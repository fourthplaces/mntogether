//! Generate embedding action - creates embedding vector for member's searchable text

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::domains::chatrooms::ChatRequestState;
use crate::domains::member::events::MemberEvent;
use crate::domains::member::models::member::Member;
use crate::domains::posts::effects::ServerDeps;

/// Generate embedding for a member's searchable text.
///
/// This action:
/// 1. Loads the member from database
/// 2. Generates embedding using the embedding service
/// 3. Stores the embedding in the database
///
/// Returns:
/// - `EmbeddingGenerated` on success
/// - `EmbeddingFailed` if member not found or generation fails
pub async fn generate_embedding(
    member_id: Uuid,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<MemberEvent> {
    info!("Generating embedding for member: {}", member_id);

    // Get member from database
    let member = match Member::find_by_id(member_id, &ctx.deps().db_pool).await {
        Ok(m) => m,
        Err(e) => {
            error!("Member not found: {}", e);
            return Ok(MemberEvent::EmbeddingFailed {
                member_id,
                reason: format!("Member not found: {}", e),
            });
        }
    };

    // Generate embedding using the embedding service
    let embedding = match ctx
        .deps()
        .embedding_service
        .generate(&member.searchable_text)
        .await
    {
        Ok(emb) => emb,
        Err(e) => {
            error!("Failed to generate embedding: {}", e);
            return Ok(MemberEvent::EmbeddingFailed {
                member_id,
                reason: format!("Embedding generation failed: {}", e),
            });
        }
    };

    debug!("Generated embedding with {} dimensions", embedding.len());

    // Update member with embedding
    if let Err(e) = Member::update_embedding(member_id, &embedding, &ctx.deps().db_pool).await {
        error!("Failed to save embedding: {}", e);
        return Ok(MemberEvent::EmbeddingFailed {
            member_id,
            reason: format!("Failed to save embedding: {}", e),
        });
    }

    info!("Embedding generated and saved for member: {}", member_id);

    Ok(MemberEvent::EmbeddingGenerated {
        member_id,
        dimensions: embedding.len(),
    })
}
