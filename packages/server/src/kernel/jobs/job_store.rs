//! Implementation of seesaw's JobStore trait for PostgresJobQueue.
//!
//! This module bridges seesaw's job interface with our PostgreSQL storage.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::job::{ClaimedJob, FailureKind, JobStore};
use uuid::Uuid;

use super::job::{ErrorKind, Job};
use super::queue::PostgresJobQueue;
use crate::common::sql::Record;
use crate::kernel::ServerKernel;

/// Wrapper around PostgresJobQueue that implements seesaw's JobStore trait.
pub struct PostgresJobStore {
    kernel: Arc<ServerKernel>,
    default_lease_ms: i64,
}

impl PostgresJobStore {
    /// Create a new PostgresJobStore.
    pub fn new(kernel: Arc<ServerKernel>) -> Self {
        Self {
            kernel,
            default_lease_ms: 60_000,
        }
    }

    /// Create from an existing PostgresJobQueue.
    pub fn from_queue(queue: &PostgresJobQueue) -> Self {
        Self {
            kernel: Arc::clone(queue.kernel()),
            default_lease_ms: queue.default_lease_ms(),
        }
    }

    /// Convert an api-core Job to seesaw ClaimedJob.
    fn to_claimed_job(job: Job) -> ClaimedJob {
        ClaimedJob {
            id: job.id,
            job_type: job.job_type,
            payload: job.args.unwrap_or(serde_json::Value::Null),
            version: job.command_version,
            attempt: job.attempt,
        }
    }
}

#[async_trait]
impl JobStore for PostgresJobStore {
    async fn claim_ready(&self, worker_id: &str, limit: i64) -> Result<Vec<ClaimedJob>> {
        let jobs = Job::claim_jobs(limit, worker_id, self.default_lease_ms, &self.kernel).await?;
        Ok(jobs.into_iter().map(Self::to_claimed_job).collect())
    }

    async fn mark_succeeded(&self, job_id: Uuid) -> Result<()> {
        // Fetch the job to check if it's recurring
        let job = Job::find_by_id(job_id, &self.kernel.db_connection).await?;

        // Handle recurring jobs - schedule next occurrence
        if job.frequency.is_some() {
            if let Ok(Some(next_run)) = job.calculate_next_run_at_from_frequency() {
                sqlx::query(
                    r#"
                    UPDATE jobs
                    SET status = 'pending',
                        next_run_at = $1,
                        last_run_at = NOW(),
                        retry_count = 0,
                        attempt = 1,
                        error_message = NULL,
                        error_kind = NULL,
                        lease_expires_at = NULL,
                        worker_id = NULL,
                        dedupe_key = $2,
                        updated_at = NOW()
                    WHERE id = $3
                    "#,
                )
                .bind(next_run)
                .bind(format!("{}:{}", job.reference_id, next_run.timestamp()))
                .bind(job_id)
                .execute(&self.kernel.db_connection)
                .await?;

                return Ok(());
            }
        }

        // Non-recurring or no next occurrence - mark as succeeded
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'succeeded',
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .execute(&self.kernel.db_connection)
        .await?;

        Ok(())
    }

    async fn mark_failed(&self, job_id: Uuid, error: &str, kind: FailureKind) -> Result<()> {
        let error_kind = ErrorKind::from(kind);

        // Fetch current job state
        let job = Job::find_by_id(job_id, &self.kernel.db_connection).await?;

        if error_kind.should_retry() && job.retry_count < job.max_retries {
            // Schedule retry with exponential backoff
            let delay_secs = 2i64.pow(job.retry_count as u32).min(3600); // Max 1 hour
            let retry_at = chrono::Utc::now() + chrono::Duration::seconds(delay_secs);

            let retry_job = job.create_retry(retry_at);
            retry_job.insert(&self.kernel).await?;

            // Mark original as failed
            sqlx::query(
                r#"
                UPDATE jobs
                SET status = 'failed',
                    error_message = $1,
                    error_kind = $2,
                    updated_at = NOW()
                WHERE id = $3
                "#,
            )
            .bind(error)
            .bind(error_kind)
            .bind(job_id)
            .execute(&self.kernel.db_connection)
            .await?;
        } else {
            // No retries left - dead letter
            sqlx::query(
                r#"
                UPDATE jobs
                SET status = 'dead_letter',
                    error_message = $1,
                    error_kind = $2,
                    dead_lettered_at = NOW(),
                    dead_letter_reason = 'max retries exceeded',
                    updated_at = NOW()
                WHERE id = $3
                "#,
            )
            .bind(error)
            .bind(error_kind)
            .bind(job_id)
            .execute(&self.kernel.db_connection)
            .await?;
        }

        Ok(())
    }

    async fn heartbeat(&self, job_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET lease_expires_at = NOW() + ($1 || ' milliseconds')::INTERVAL,
                updated_at = NOW()
            WHERE id = $2 AND status = 'running'
            "#,
        )
        .bind(self.default_lease_ms.to_string())
        .bind(job_id)
        .execute(&self.kernel.db_connection)
        .await?;

        Ok(())
    }
}
