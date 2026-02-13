//! SyncProposalMergeSource model - entities to absorb in MERGE proposals

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::SyncProposalId;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SyncProposalMergeSource {
    pub id: Uuid,
    pub proposal_id: SyncProposalId,
    pub source_entity_id: Uuid,
}

impl SyncProposalMergeSource {
    pub async fn create(
        proposal_id: SyncProposalId,
        source_entity_id: Uuid,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sync_proposal_merge_sources (proposal_id, source_entity_id)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(proposal_id)
        .bind(source_entity_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_proposal(proposal_id: SyncProposalId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sync_proposal_merge_sources WHERE proposal_id = $1",
        )
        .bind(proposal_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
