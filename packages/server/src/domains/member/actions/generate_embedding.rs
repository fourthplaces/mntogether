//! Generate embedding action - creates embedding vector for member's searchable text

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::common::AppState;
use crate::domains::member::events::MemberEvent;
use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Generate embedding for a member's searchable text - emits events directly.
///
/// This action:
/// 1. Loads the member from database
/// 2. Generates embedding using the embedding service
/// 3. Stores the embedding in the database
///
/// Emits:
/// - `EmbeddingGenerated` on success
/// - `EmbeddingFailed` if member not found or generation fails
pub async fn handle_generate_embedding(
    member_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!("Generating embedding for member: {}", member_id);

    // Get member from database
    let member = match Member::find_by_id(member_id, &ctx.deps().db_pool).await {
        Ok(m) => m,
        Err(e) => {
            error!("Member not found: {}", e);
            ctx.emit(MemberEvent::EmbeddingFailed {
                member_id,
                reason: format!("Member not found: {}", e),
            });
            return Ok(());
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
            ctx.emit(MemberEvent::EmbeddingFailed {
                member_id,
                reason: format!("Embedding generation failed: {}", e),
            });
            return Ok(());
        }
    };

    debug!("Generated embedding with {} dimensions", embedding.len());

    // Update member with embedding
    if let Err(e) = Member::update_embedding(member_id, &embedding, &ctx.deps().db_pool).await {
        error!("Failed to save embedding: {}", e);
        ctx.emit(MemberEvent::EmbeddingFailed {
            member_id,
            reason: format!("Failed to save embedding: {}", e),
        });
        return Ok(());
    }

    info!("Embedding generated and saved for member: {}", member_id);

    ctx.emit(MemberEvent::EmbeddingGenerated {
        member_id,
        dimensions: embedding.len(),
    });
    Ok(())
}
