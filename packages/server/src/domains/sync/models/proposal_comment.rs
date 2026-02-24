//! ProposalComment model - admin feedback on sync proposals for AI-driven refinement.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{MemberId, ProposalCommentId, SyncProposalId};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProposalComment {
    pub id: ProposalCommentId,
    pub proposal_id: SyncProposalId,
    pub author_id: MemberId,
    pub content: String,
    pub revision_number: i32,
    pub ai_revised: bool,
    pub created_at: DateTime<Utc>,
}

impl ProposalComment {
    pub async fn create(
        proposal_id: SyncProposalId,
        author_id: MemberId,
        content: &str,
        revision_number: i32,
        ai_revised: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO proposal_comments (proposal_id, author_id, content, revision_number, ai_revised)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(proposal_id)
        .bind(author_id)
        .bind(content)
        .bind(revision_number)
        .bind(ai_revised)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_proposal(
        proposal_id: SyncProposalId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM proposal_comments WHERE proposal_id = $1 ORDER BY created_at ASC",
        )
        .bind(proposal_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn count_for_proposal(
        proposal_id: SyncProposalId,
        pool: &PgPool,
    ) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM proposal_comments WHERE proposal_id = $1",
        )
        .bind(proposal_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
