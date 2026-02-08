//! SyncProposal model - individual proposed operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{MemberId, SyncBatchId, SyncProposalId};
use crate::impl_restate_serde;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncProposal {
    pub id: SyncProposalId,
    pub batch_id: SyncBatchId,
    pub operation: String,
    pub status: String,
    pub entity_type: String,
    pub draft_entity_id: Option<Uuid>,
    pub target_entity_id: Option<Uuid>,
    pub reason: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl_restate_serde!(SyncProposal);

/// Input for creating a new sync proposal
pub struct CreateSyncProposal {
    pub batch_id: SyncBatchId,
    pub operation: String,
    pub entity_type: String,
    pub draft_entity_id: Option<Uuid>,
    pub target_entity_id: Option<Uuid>,
    pub reason: Option<String>,
}

impl SyncProposal {
    pub async fn create(input: CreateSyncProposal, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sync_proposals (batch_id, operation, entity_type, draft_entity_id, target_entity_id, reason)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(input.batch_id)
        .bind(&input.operation)
        .bind(&input.entity_type)
        .bind(input.draft_entity_id)
        .bind(input.target_entity_id)
        .bind(&input.reason)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: SyncProposalId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM sync_proposals WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_batch(batch_id: SyncBatchId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sync_proposals
            WHERE batch_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(batch_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_pending_by_batch(batch_id: SyncBatchId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sync_proposals
            WHERE batch_id = $1 AND status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .bind(batch_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn approve(id: SyncProposalId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sync_proposals
            SET status = 'approved', reviewed_by = $2, reviewed_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by.into_uuid())
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn reject(id: SyncProposalId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sync_proposals
            SET status = 'rejected', reviewed_by = $2, reviewed_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by.into_uuid())
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn count_pending(batch_id: SyncBatchId, pool: &PgPool) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sync_proposals WHERE batch_id = $1 AND status = 'pending'",
        )
        .bind(batch_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all pending proposals for a given entity (as target or draft),
    /// excluding proposals in expired or completed batches.
    pub async fn find_pending_for_entity(entity_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT sp.* FROM sync_proposals sp
            INNER JOIN sync_batches sb ON sb.id = sp.batch_id
            WHERE (sp.target_entity_id = $1 OR sp.draft_entity_id = $1)
              AND sp.status = 'pending'
              AND sb.status IN ('pending', 'partially_reviewed')
            ORDER BY sp.created_at DESC
            "#,
        )
        .bind(entity_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Reject all pending proposals in an expired batch
    pub async fn reject_all_pending(batch_id: SyncBatchId, pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sync_proposals
            SET status = 'rejected', reviewed_at = NOW()
            WHERE batch_id = $1 AND status = 'pending'
            "#,
        )
        .bind(batch_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
