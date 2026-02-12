use anyhow::Result;
use sqlx::PgPool;

use crate::common::NoteId;
use crate::domains::notes::models::Note;
use crate::domains::sync::activities::ProposalHandler;
use crate::domains::sync::models::{SyncProposal, SyncProposalMergeSource};

pub struct NoteProposalHandler;

impl ProposalHandler for NoteProposalHandler {
    fn entity_type(&self) -> &str {
        "note"
    }

    async fn approve(
        &self,
        proposal: &SyncProposal,
        _merge_sources: &[SyncProposalMergeSource],
        pool: &PgPool,
    ) -> Result<()> {
        match proposal.operation.as_str() {
            "insert" => {
                if let Some(draft_id) = proposal.draft_entity_id {
                    Note::activate(NoteId::from(draft_id), pool).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn reject(
        &self,
        proposal: &SyncProposal,
        _merge_sources: &[SyncProposalMergeSource],
        pool: &PgPool,
    ) -> Result<()> {
        if let Some(draft_id) = proposal.draft_entity_id {
            Note::delete(NoteId::from(draft_id), pool).await?;
        }
        Ok(())
    }
}
