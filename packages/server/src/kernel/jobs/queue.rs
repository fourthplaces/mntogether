//! PostgreSQL-backed job queue implementation.
//!
//! This module provides the core job queue functionality for storing
//! and retrieving jobs from PostgreSQL.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Serialize, de::DeserializeOwned};
use tracing::info;
use uuid::Uuid;

use super::job::{ErrorKind, Job, JobPriority};
use crate::common::sql::Record;
use crate::kernel::ServerKernel;

/// Result type for enqueue operations that handles idempotency.
#[derive(Debug, Clone)]
pub enum EnqueueResult {
    /// Command was enqueued, returns new job ID
    Created(Uuid),
    /// Command already exists (idempotency hit), returns existing job ID
    Duplicate(Uuid),
}

impl EnqueueResult {
    /// Get the job ID regardless of whether it was created or duplicate
    pub fn job_id(&self) -> Uuid {
        match self {
            EnqueueResult::Created(id) | EnqueueResult::Duplicate(id) => *id,
        }
    }

    /// Returns true if this was a newly created job
    pub fn is_created(&self) -> bool {
        matches!(self, EnqueueResult::Created(_))
    }
}

/// A claimed job ready for execution.
#[derive(Debug)]
pub struct ClaimedJob {
    /// The job ID
    pub id: Uuid,
    /// The raw job record
    pub job: Job,
}

impl ClaimedJob {
    /// Deserialize the command payload.
    pub fn deserialize<C: DeserializeOwned>(&self) -> Result<C> {
        let args = self
            .job
            .args
            .as_ref()
            .ok_or_else(|| anyhow!("job {} has no args", self.id))?;
        serde_json::from_value(args.clone())
            .map_err(|e| anyhow!("failed to deserialize command: {}", e))
    }

    /// Get the command type (job_type)
    pub fn command_type(&self) -> &str {
        &self.job.job_type
    }

    /// Get the command version
    pub fn command_version(&self) -> i32 {
        self.job.command_version
    }
}

/// Metadata for command serialization.
///
/// Commands should implement this trait to provide type information
/// and optional idempotency keys.
pub trait CommandMeta {
    /// The command type name (used as job_type).
    fn command_type(&self) -> &'static str;

    /// Optional idempotency key.
    ///
    /// If provided, ensures only one pending/running job exists with this key.
    fn idempotency_key(&self) -> Option<String> {
        None
    }

    /// The command version for schema evolution.
    fn command_version(&self) -> i32 {
        1
    }

    /// Optional priority override.
    fn priority(&self) -> JobPriority {
        JobPriority::Normal
    }

    /// Optional reference ID for the job.
    ///
    /// Defaults to a new UUID if not provided.
    fn reference_id(&self) -> Option<Uuid> {
        None
    }

    /// Optional container scope.
    fn container_id(&self) -> Option<Uuid> {
        None
    }

    /// Maximum retries for this command.
    fn max_retries(&self) -> i32 {
        3
    }
}

/// Trait for job queue operations.
///
/// Implementations provide the storage and retrieval of serialized Commands
/// for background execution.
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue a command for immediate execution.
    ///
    /// If the command provides an idempotency key and a matching pending/running
    /// job exists, returns `EnqueueResult::Duplicate` with the existing job ID.
    async fn enqueue<C>(&self, command: C) -> Result<EnqueueResult>
    where
        C: Serialize + Send + CommandMeta;

    /// Schedule a command for future execution.
    async fn schedule<C>(&self, command: C, run_at: DateTime<Utc>) -> Result<EnqueueResult>
    where
        C: Serialize + Send + CommandMeta;

    /// Claim up to `limit` jobs for processing.
    ///
    /// Uses `FOR UPDATE SKIP LOCKED` for concurrent-safe claiming.
    /// Returns claimed jobs with their serialized command payloads.
    async fn claim(&self, worker_id: &str, limit: i64) -> Result<Vec<ClaimedJob>>;

    /// Mark a job as successfully completed.
    async fn mark_succeeded(&self, job_id: Uuid) -> Result<()>;

    /// Mark a job as failed with an error.
    ///
    /// If retries remain, the job will be re-queued for retry.
    /// Otherwise, it will be moved to dead letter.
    async fn mark_failed(&self, job_id: Uuid, error: &str, kind: ErrorKind) -> Result<()>;

    /// Cancel a pending job.
    ///
    /// Only cancels jobs in pending status. Running jobs should be
    /// cancelled via cooperative cancellation token.
    async fn cancel(&self, job_id: Uuid) -> Result<bool>;

    /// Extend the lease for a running job (heartbeat).
    async fn heartbeat(&self, job_id: Uuid) -> Result<()>;

    /// Find the next scheduled run time (for sleep optimization).
    async fn next_run_time(&self) -> Result<Option<DateTime<Utc>>>;
}

/// PostgreSQL-backed job queue implementation.
pub struct PostgresJobQueue {
    kernel: Arc<ServerKernel>,
    default_lease_ms: i64,
}

impl PostgresJobQueue {
    /// Create a new PostgreSQL job queue.
    pub fn new(kernel: Arc<ServerKernel>) -> Self {
        Self {
            kernel,
            default_lease_ms: 60_000, // 1 minute
        }
    }

    /// Create with a custom lease duration.
    pub fn with_lease_duration(kernel: Arc<ServerKernel>, lease_ms: i64) -> Self {
        Self {
            kernel,
            default_lease_ms: lease_ms,
        }
    }

    /// Get the default lease duration in milliseconds.
    pub fn default_lease_ms(&self) -> i64 {
        self.default_lease_ms
    }

    /// Get a reference to the kernel.
    pub fn kernel(&self) -> &Arc<ServerKernel> {
        &self.kernel
    }

    /// Check if a job with the given idempotency key already exists.
    pub async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Job>> {
        let job = sqlx::query_as::<_, Job>(
            r#"
            SELECT id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                   max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                   args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                   lease_expires_at, worker_id, enabled, error_message, error_kind,
                   dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                   root_job_id, dedupe_key, attempt, idempotency_key, command_version
            FROM jobs
            WHERE idempotency_key = $1
              AND status IN ('pending', 'running')
            LIMIT 1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.kernel.db_connection)
        .await?;

        Ok(job)
    }

    /// Internal method to enqueue a command using JobSpec.
    pub async fn enqueue_with_spec(
        &self,
        payload: serde_json::Value,
        job_type: &str,
        reference_id: Option<Uuid>,
        idempotency_key: Option<String>,
        command_version: i32,
        priority: JobPriority,
        max_retries: i32,
        container_id: Option<Uuid>,
        run_at: Option<DateTime<Utc>>,
    ) -> Result<EnqueueResult> {
        // Check idempotency first
        if let Some(key) = &idempotency_key {
            if let Some(existing) = self.find_by_idempotency_key(key).await? {
                return Ok(EnqueueResult::Duplicate(existing.id));
            }
        }

        // Build job using the command constructor
        let job = Job::for_command(
            job_type,
            payload,
            reference_id,
            run_at,
            idempotency_key,
            command_version,
            priority,
            max_retries,
            container_id,
            self.default_lease_ms,
        );

        // Insert
        let inserted = job.insert(&self.kernel).await?;

        Ok(EnqueueResult::Created(inserted.id))
    }

    /// Mark a job as successfully completed.
    pub async fn mark_succeeded(&self, job_id: Uuid) -> Result<()> {
        // Fetch the job to check if it's recurring
        let job = Job::find_by_id(job_id, &self.kernel.db_connection).await?;

        // Handle recurring jobs - schedule next occurrence
        if job.frequency.is_some() {
            if let Ok(Some(next_run)) = job.calculate_next_run_at_from_frequency() {
                // Update job for next occurrence instead of marking succeeded
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

                info!(
                    job_id = %job_id,
                    next_run_at = %next_run,
                    "scheduled next occurrence for recurring job"
                );

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

    /// Mark a job as failed with an error.
    pub async fn mark_failed(&self, job_id: Uuid, error: &str, kind: ErrorKind) -> Result<()> {
        // Fetch current job state
        let job = Job::find_by_id(job_id, &self.kernel.db_connection).await?;

        if kind.should_retry() && job.retry_count < job.max_retries {
            // Schedule retry with exponential backoff
            let delay_secs = 2i64.pow(job.retry_count as u32).min(3600); // Max 1 hour
            let retry_at = Utc::now() + chrono::Duration::seconds(delay_secs);

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
            .bind(kind)
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
            .bind(kind)
            .bind(job_id)
            .execute(&self.kernel.db_connection)
            .await?;
        }

        Ok(())
    }

    /// Cancel a pending job.
    pub async fn cancel(&self, job_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'cancelled',
                error_kind = 'cancelled',
                updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            "#,
        )
        .bind(job_id)
        .execute(&self.kernel.db_connection)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Extend the lease for a running job (heartbeat).
    pub async fn heartbeat(&self, job_id: Uuid) -> Result<()> {
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

    /// Find the next scheduled run time (for sleep optimization).
    pub async fn next_run_time(&self) -> Result<Option<DateTime<Utc>>> {
        Job::find_next_run_time(&self.kernel).await
    }

    /// Claim jobs for processing (internal, returns raw Jobs).
    pub async fn claim_jobs_internal(&self, worker_id: &str, limit: i64) -> Result<Vec<Job>> {
        Job::claim_jobs(limit, worker_id, self.default_lease_ms, &self.kernel).await
    }
}

#[async_trait]
impl JobQueue for PostgresJobQueue {
    async fn enqueue<C>(&self, command: C) -> Result<EnqueueResult>
    where
        C: Serialize + Send + CommandMeta,
    {
        self.enqueue_internal(command, None).await
    }

    async fn schedule<C>(&self, command: C, run_at: DateTime<Utc>) -> Result<EnqueueResult>
    where
        C: Serialize + Send + CommandMeta,
    {
        self.enqueue_internal(command, Some(run_at)).await
    }

    async fn claim(&self, worker_id: &str, limit: i64) -> Result<Vec<ClaimedJob>> {
        let jobs = Job::claim_jobs(limit, worker_id, self.default_lease_ms, &self.kernel).await?;

        Ok(jobs
            .into_iter()
            .map(|job| ClaimedJob { id: job.id, job })
            .collect())
    }

    async fn mark_succeeded(&self, job_id: Uuid) -> Result<()> {
        PostgresJobQueue::mark_succeeded(self, job_id).await
    }

    async fn mark_failed(&self, job_id: Uuid, error: &str, kind: ErrorKind) -> Result<()> {
        PostgresJobQueue::mark_failed(self, job_id, error, kind).await
    }

    async fn cancel(&self, job_id: Uuid) -> Result<bool> {
        PostgresJobQueue::cancel(self, job_id).await
    }

    async fn heartbeat(&self, job_id: Uuid) -> Result<()> {
        PostgresJobQueue::heartbeat(self, job_id).await
    }

    async fn next_run_time(&self) -> Result<Option<DateTime<Utc>>> {
        PostgresJobQueue::next_run_time(self).await
    }
}

impl PostgresJobQueue {
    /// Internal method to enqueue a command.
    async fn enqueue_internal<C>(
        &self,
        command: C,
        run_at: Option<DateTime<Utc>>,
    ) -> Result<EnqueueResult>
    where
        C: Serialize + Send + CommandMeta,
    {
        // Check idempotency first
        if let Some(key) = command.idempotency_key() {
            if let Some(existing) = self.find_by_idempotency_key(&key).await? {
                return Ok(EnqueueResult::Duplicate(existing.id));
            }
        }

        // Serialize command to JSON
        let args = serde_json::to_value(&command)?;

        // Build job using the command constructor
        let command_type = command.command_type();
        let reference_id = command.reference_id();
        let container_id = command.container_id();

        let job = Job::for_command(
            command_type,
            args,
            reference_id,
            run_at,
            command.idempotency_key(),
            command.command_version(),
            command.priority(),
            command.max_retries(),
            container_id,
            self.default_lease_ms,
        );

        // Insert (let DB handle idempotency constraint as backup)
        let inserted = job.insert(&self.kernel).await?;

        // In test mode, also notify the TestJobManager for test assertions
        if self.kernel.test_mode {
            use super::manager::ScheduleOptions;

            let test_options = ScheduleOptions::builder()
                .reference_id(reference_id.unwrap_or(inserted.id))
                .job_type(command_type)
                .run_at(run_at)
                .container_id(container_id)
                .build();

            // The job_manager schedule will store it in memory for TestJobManager
            let _ = self.kernel.job_manager.schedule(test_options).await;
        }

        // Publish to NATS for job workers
        let subject = format!("jobs.{}", command_type);
        let payload = serde_json::to_vec(&inserted)?;
        self.kernel
            .nats_publisher
            .publish(subject, payload.into())
            .await?;

        Ok(EnqueueResult::Created(inserted.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_result_helpers() {
        let created = EnqueueResult::Created(Uuid::new_v4());
        assert!(created.is_created());

        let duplicate = EnqueueResult::Duplicate(Uuid::new_v4());
        assert!(!duplicate.is_created());
    }
}
