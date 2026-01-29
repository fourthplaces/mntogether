//! Adapter for seesaw's JobQueue trait using api-core's PostgreSQL storage.
//!
//! This module provides a bridge between seesaw's job queue abstraction and
//! api-core's PostgreSQL-based job queue implementation.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use seesaw_core::JobSpec;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;

use super::job::{Job, JobPriority};

/// Adapter that implements seesaw's `JobQueue` trait using PostgreSQL.
///
/// This adapter receives pre-serialized command payloads from the seesaw
/// dispatcher and stores them in the PostgreSQL job queue. The `JobSpec`
/// provides all the metadata needed for job execution.
pub struct SeesawJobQueueAdapter {
    db: PgPool,
    default_lease_ms: i64,
}

impl SeesawJobQueueAdapter {
    /// Create a new adapter using the provided database pool.
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            default_lease_ms: 60_000, // 1 minute default lease
        }
    }

    /// Check if a job with the given idempotency key already exists.
    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Job>> {
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
        .fetch_optional(&self.db)
        .await?;

        Ok(job)
    }

    /// Internal method to enqueue/schedule a command.
    async fn enqueue_internal(
        &self,
        payload: serde_json::Value,
        spec: JobSpec,
        run_at: Option<DateTime<Utc>>,
    ) -> Result<Uuid> {
        // Check idempotency first
        if let Some(key) = &spec.idempotency_key {
            if let Some(existing) = self.find_by_idempotency_key(key).await? {
                debug!(
                    job_id = %existing.id,
                    idempotency_key = %key,
                    "found existing job with idempotency key"
                );
                return Ok(existing.id);
            }
        }

        // Convert priority from i32 to JobPriority
        let priority = match spec.priority {
            p if p >= 3 => JobPriority::Critical,
            p if p >= 1 => JobPriority::High,
            p if p <= -2 => JobPriority::Low,
            _ => JobPriority::Normal,
        };

        // Build job using the command constructor
        let job = Job::for_command(
            spec.job_type,
            payload,
            spec.reference_id,
            run_at,
            spec.idempotency_key.clone(),
            spec.version,
            priority,
            spec.max_retries,
            None, // container_id - not part of JobSpec, used internally by job system
            self.default_lease_ms,
        );

        debug!(
            job_id = %job.id,
            job_type = %spec.job_type,
            run_at = ?run_at,
            "enqueueing job via seesaw adapter"
        );

        // Insert into database
        let inserted = job.insert_with_pool(&self.db).await?;

        Ok(inserted.id)
    }
}

#[async_trait]
impl seesaw::JobQueue for SeesawJobQueueAdapter {
    async fn enqueue(&self, payload: serde_json::Value, spec: JobSpec) -> Result<Uuid> {
        self.enqueue_internal(payload, spec, None).await
    }

    async fn schedule(
        &self,
        payload: serde_json::Value,
        spec: JobSpec,
        run_at: DateTime<Utc>,
    ) -> Result<Uuid> {
        self.enqueue_internal(payload, spec, Some(run_at)).await
    }
}
