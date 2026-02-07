//! Post-specific proposal approval/rejection logic
//!
//! Implements `ProposalHandler` for entity_type = "post".
//! Dispatches to the existing post model and revision actions.

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::common::PostId;
use crate::domains::posts::models::Post;
use crate::domains::sync::activities::ProposalHandler;
use crate::domains::sync::models::{SyncProposal, SyncProposalMergeSource};

use super::revision_actions;

pub struct PostProposalHandler;

impl ProposalHandler for PostProposalHandler {
    fn entity_type(&self) -> &str {
        "post"
    }

    async fn approve(
        &self,
        proposal: &SyncProposal,
        merge_sources: &[SyncProposalMergeSource],
        pool: &PgPool,
    ) -> Result<()> {
        match proposal.operation.as_str() {
            "insert" => approve_insert(proposal, pool).await,
            "update" => approve_update(proposal, pool).await,
            "delete" => approve_delete(proposal, pool).await,
            "merge" => approve_merge(proposal, merge_sources, pool).await,
            other => anyhow::bail!("Unknown operation type: {}", other),
        }
    }

    async fn reject(
        &self,
        proposal: &SyncProposal,
        _merge_sources: &[SyncProposalMergeSource],
        pool: &PgPool,
    ) -> Result<()> {
        match proposal.operation.as_str() {
            "insert" => reject_insert(proposal, pool).await,
            "update" => reject_update(proposal, pool).await,
            "delete" | "merge" => Ok(()), // no-op: nothing was changed at staging time
            other => anyhow::bail!("Unknown operation type: {}", other),
        }
    }
}

/// Approve INSERT: set draft post status to active
async fn approve_insert(proposal: &SyncProposal, pool: &PgPool) -> Result<()> {
    let draft_id = proposal
        .draft_entity_id
        .ok_or_else(|| anyhow::anyhow!("Insert proposal missing draft_entity_id"))?;
    let post_id = PostId::from(draft_id);

    Post::update_status(post_id, "active", pool).await?;

    info!(post_id = %post_id, "Approved insert: post set to active");
    Ok(())
}

/// Approve UPDATE: apply the revision to the original post
async fn approve_update(proposal: &SyncProposal, pool: &PgPool) -> Result<()> {
    let revision_id = proposal
        .draft_entity_id
        .ok_or_else(|| anyhow::anyhow!("Update proposal missing draft_entity_id (revision)"))?;
    let post_id = PostId::from(revision_id);

    revision_actions::approve_revision(post_id, pool).await?;

    info!(revision_id = %revision_id, "Approved update: revision applied to original");
    Ok(())
}

/// Approve DELETE: soft-delete the target post
async fn approve_delete(proposal: &SyncProposal, pool: &PgPool) -> Result<()> {
    let target_id = proposal
        .target_entity_id
        .ok_or_else(|| anyhow::anyhow!("Delete proposal missing target_entity_id"))?;
    let post_id = PostId::from(target_id);

    let reason = proposal
        .reason
        .as_deref()
        .unwrap_or("Approved deletion from sync proposal");

    Post::soft_delete(post_id, reason, pool).await?;

    info!(post_id = %post_id, "Approved delete: post soft-deleted");
    Ok(())
}

/// Approve MERGE: apply revision (if any) to canonical, soft-delete merge sources
async fn approve_merge(
    proposal: &SyncProposal,
    merge_sources: &[SyncProposalMergeSource],
    pool: &PgPool,
) -> Result<()> {
    let canonical_id = proposal
        .target_entity_id
        .ok_or_else(|| anyhow::anyhow!("Merge proposal missing target_entity_id (canonical)"))?;

    // If there's a draft (revision with merged content), apply it
    if let Some(revision_uuid) = proposal.draft_entity_id {
        let revision_id = PostId::from(revision_uuid);
        revision_actions::approve_revision(revision_id, pool).await?;
        info!(revision_id = %revision_id, canonical_id = %canonical_id, "Applied merged content revision");
    }

    // Soft-delete all merge sources
    let reason = proposal
        .reason
        .as_deref()
        .unwrap_or("Merged into canonical post");

    for source in merge_sources {
        let source_post_id = PostId::from(source.source_entity_id);
        Post::soft_delete(source_post_id, reason, pool).await?;
        info!(source_id = %source.source_entity_id, canonical_id = %canonical_id, "Soft-deleted merge source");
    }

    Ok(())
}

/// Reject INSERT: delete the draft post
async fn reject_insert(proposal: &SyncProposal, pool: &PgPool) -> Result<()> {
    let draft_id = proposal
        .draft_entity_id
        .ok_or_else(|| anyhow::anyhow!("Insert proposal missing draft_entity_id"))?;
    let post_id = PostId::from(draft_id);

    Post::delete(post_id, pool).await?;

    info!(post_id = %post_id, "Rejected insert: draft post deleted");
    Ok(())
}

/// Reject UPDATE: delete the revision post
async fn reject_update(proposal: &SyncProposal, pool: &PgPool) -> Result<()> {
    let revision_id = proposal
        .draft_entity_id
        .ok_or_else(|| anyhow::anyhow!("Update proposal missing draft_entity_id (revision)"))?;
    let post_id = PostId::from(revision_id);

    revision_actions::reject_revision(post_id, pool).await?;

    info!(revision_id = %revision_id, "Rejected update: revision deleted");
    Ok(())
}
