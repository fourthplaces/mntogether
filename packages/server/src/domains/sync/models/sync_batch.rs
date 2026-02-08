//! SyncBatch model - groups proposals from one AI operation

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::SyncBatchId;
use crate::impl_restate_serde;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncBatch {
    pub id: SyncBatchId,
    pub resource_type: String,
    pub source_id: Option<Uuid>,
    pub status: String,
    pub summary: Option<String>,
    pub proposal_count: i32,
    pub approved_count: i32,
    pub rejected_count: i32,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl_restate_serde!(SyncBatch);

impl SyncBatch {
    pub async fn create(
        resource_type: &str,
        source_id: Option<Uuid>,
        summary: Option<&str>,
        proposal_count: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sync_batches (resource_type, source_id, summary, proposal_count)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(resource_type)
        .bind(source_id)
        .bind(summary)
        .bind(proposal_count)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: SyncBatchId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM sync_batches WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_pending_by_resource(
        resource_type: &str,
        source_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sync_batches
            WHERE resource_type = $1
              AND source_id = $2
              AND status = 'pending'
            ORDER BY created_at DESC
            "#,
        )
        .bind(resource_type)
        .bind(source_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_pending(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sync_batches
            WHERE status IN ('pending', 'partially_reviewed')
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_status(id: SyncBatchId, status: &str, pool: &PgPool) -> Result<Self> {
        let reviewed_at = if status == "completed" || status == "expired" {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sync_batches
            SET status = $2,
                reviewed_at = COALESCE($3, reviewed_at)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(reviewed_at)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn increment_approved(id: SyncBatchId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sync_batches
            SET approved_count = approved_count + 1
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn increment_rejected(id: SyncBatchId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sync_batches
            SET rejected_count = rejected_count + 1
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_recent(limit: i32, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sync_batches
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Expire stale pending batches for the same resource_type + source_id.
    /// Called before creating a new batch to avoid reviewing outdated proposals.
    pub async fn expire_stale(resource_type: &str, source_id: Uuid, pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sync_batches
            SET status = 'expired', reviewed_at = NOW()
            WHERE resource_type = $1
              AND source_id = $2
              AND status IN ('pending', 'partially_reviewed')
            "#,
        )
        .bind(resource_type)
        .bind(source_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
