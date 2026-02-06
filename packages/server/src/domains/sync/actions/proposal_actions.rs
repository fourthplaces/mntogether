//! Generic sync proposal actions
//!
//! Handles staging, approving, and rejecting AI-proposed changes.
//! Entity-specific logic is dispatched via the `ProposalHandler` trait.

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::{MemberId, SyncBatchId, SyncProposalId};
use crate::domains::sync::models::{
    CreateSyncProposal, SyncBatch, SyncProposal, SyncProposalMergeSource,
};

/// A single proposed operation to be staged
pub struct ProposedOperation {
    pub operation: String,
    pub entity_type: String,
    pub draft_entity_id: Option<Uuid>,
    pub target_entity_id: Option<Uuid>,
    pub reason: Option<String>,
    /// For merge operations: IDs of entities to absorb into the target
    pub merge_source_ids: Vec<Uuid>,
}

/// Result of staging proposals
pub struct StageResult {
    pub batch_id: SyncBatchId,
    pub proposal_count: usize,
    pub expired_batches: u64,
}

/// Stage a set of proposals into a new batch.
///
/// Automatically expires stale pending batches for the same resource + source.
pub async fn stage_proposals(
    resource_type: &str,
    source_id: Uuid,
    summary: Option<&str>,
    proposals: Vec<ProposedOperation>,
    pool: &PgPool,
) -> Result<StageResult> {
    // Expire stale batches first
    let expired = SyncBatch::expire_stale(resource_type, source_id, pool).await?;
    if expired > 0 {
        info!(
            resource_type = %resource_type,
            source_id = %source_id,
            expired = expired,
            "Expired stale pending batches"
        );
    }

    let proposal_count = proposals.len();

    let batch = SyncBatch::create(
        resource_type,
        Some(source_id),
        summary,
        proposal_count as i32,
        pool,
    )
    .await?;

    info!(
        batch_id = %batch.id,
        resource_type = %resource_type,
        source_id = %source_id,
        proposal_count = proposal_count,
        "Created sync batch"
    );

    for proposed in &proposals {
        let proposal = SyncProposal::create(
            CreateSyncProposal {
                batch_id: batch.id,
                operation: proposed.operation.clone(),
                entity_type: proposed.entity_type.clone(),
                draft_entity_id: proposed.draft_entity_id,
                target_entity_id: proposed.target_entity_id,
                reason: proposed.reason.clone(),
            },
            pool,
        )
        .await?;

        // Create merge source records if this is a merge operation
        for source_id in &proposed.merge_source_ids {
            SyncProposalMergeSource::create(proposal.id, *source_id, pool).await?;
        }
    }

    Ok(StageResult {
        batch_id: batch.id,
        proposal_count,
        expired_batches: expired,
    })
}

/// Trait for entity-specific proposal approval/rejection logic.
///
/// Implementors handle what happens to the actual entities when a
/// proposal is approved or rejected.
pub trait ProposalHandler: Send + Sync {
    fn entity_type(&self) -> &str;

    fn approve(
        &self,
        proposal: &SyncProposal,
        merge_sources: &[SyncProposalMergeSource],
        pool: &PgPool,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn reject(
        &self,
        proposal: &SyncProposal,
        merge_sources: &[SyncProposalMergeSource],
        pool: &PgPool,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Approve a single proposal, dispatching to the entity-specific handler.
pub async fn approve_proposal(
    proposal_id: SyncProposalId,
    reviewed_by: MemberId,
    handler: &impl ProposalHandler,
    pool: &PgPool,
) -> Result<SyncProposal> {
    let proposal = SyncProposal::find_by_id(proposal_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;

    if proposal.status != "pending" {
        anyhow::bail!("Proposal is not pending (status: {})", proposal.status);
    }

    let merge_sources = SyncProposalMergeSource::find_by_proposal(proposal_id, pool).await?;

    // Execute entity-specific approval logic
    handler.approve(&proposal, &merge_sources, pool).await?;

    // Mark proposal as approved
    let updated = SyncProposal::approve(proposal_id, reviewed_by, pool).await?;

    // Update batch counters
    let batch = SyncBatch::increment_approved(proposal.batch_id, pool).await?;
    maybe_complete_batch(batch.id, pool).await?;

    info!(
        proposal_id = %proposal_id,
        batch_id = %proposal.batch_id,
        operation = %proposal.operation,
        "Proposal approved"
    );

    Ok(updated)
}

/// Reject a single proposal, dispatching to the entity-specific handler.
pub async fn reject_proposal(
    proposal_id: SyncProposalId,
    reviewed_by: MemberId,
    handler: &impl ProposalHandler,
    pool: &PgPool,
) -> Result<SyncProposal> {
    let proposal = SyncProposal::find_by_id(proposal_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;

    if proposal.status != "pending" {
        anyhow::bail!("Proposal is not pending (status: {})", proposal.status);
    }

    let merge_sources = SyncProposalMergeSource::find_by_proposal(proposal_id, pool).await?;

    // Execute entity-specific rejection logic
    handler.reject(&proposal, &merge_sources, pool).await?;

    // Mark proposal as rejected
    let updated = SyncProposal::reject(proposal_id, reviewed_by, pool).await?;

    // Update batch counters
    let batch = SyncBatch::increment_rejected(proposal.batch_id, pool).await?;
    maybe_complete_batch(batch.id, pool).await?;

    info!(
        proposal_id = %proposal_id,
        batch_id = %proposal.batch_id,
        operation = %proposal.operation,
        "Proposal rejected"
    );

    Ok(updated)
}

/// Approve all pending proposals in a batch.
pub async fn approve_batch(
    batch_id: SyncBatchId,
    reviewed_by: MemberId,
    handler: &impl ProposalHandler,
    pool: &PgPool,
) -> Result<SyncBatch> {
    let pending = SyncProposal::find_pending_by_batch(batch_id, pool).await?;

    for proposal in &pending {
        let merge_sources = SyncProposalMergeSource::find_by_proposal(proposal.id, pool).await?;
        handler.approve(proposal, &merge_sources, pool).await?;
        SyncProposal::approve(proposal.id, reviewed_by, pool).await?;
        SyncBatch::increment_approved(batch_id, pool).await?;
    }

    let batch = SyncBatch::update_status(batch_id, "completed", pool).await?;

    info!(
        batch_id = %batch_id,
        approved = pending.len(),
        "Batch approved"
    );

    Ok(batch)
}

/// Reject all pending proposals in a batch.
pub async fn reject_batch(
    batch_id: SyncBatchId,
    reviewed_by: MemberId,
    handler: &impl ProposalHandler,
    pool: &PgPool,
) -> Result<SyncBatch> {
    let pending = SyncProposal::find_pending_by_batch(batch_id, pool).await?;

    for proposal in &pending {
        let merge_sources = SyncProposalMergeSource::find_by_proposal(proposal.id, pool).await?;
        handler.reject(proposal, &merge_sources, pool).await?;
        SyncProposal::reject(proposal.id, reviewed_by, pool).await?;
        SyncBatch::increment_rejected(batch_id, pool).await?;
    }

    let batch = SyncBatch::update_status(batch_id, "completed", pool).await?;

    info!(
        batch_id = %batch_id,
        rejected = pending.len(),
        "Batch rejected"
    );

    Ok(batch)
}

/// Check if all proposals in a batch have been reviewed; if so, mark it completed.
async fn maybe_complete_batch(batch_id: SyncBatchId, pool: &PgPool) -> Result<()> {
    let pending = SyncProposal::count_pending(batch_id, pool).await?;

    if pending == 0 {
        SyncBatch::update_status(batch_id, "completed", pool).await?;
        info!(batch_id = %batch_id, "Batch completed - all proposals reviewed");
    } else {
        // At least one reviewed, but some pending → partially_reviewed
        let batch = SyncBatch::find_by_id(batch_id, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Batch not found"))?;
        if batch.status == "pending" {
            SyncBatch::update_status(batch_id, "partially_reviewed", pool).await?;
        }
    }

    Ok(())
}
