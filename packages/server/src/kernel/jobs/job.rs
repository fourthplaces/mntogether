//! Job model for background command execution.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::common::sql::Record;
use crate::common::utils::{calculate_next_run_at, hash::db_id, is_rrule};
use crate::kernel::ServerKernel;

// ============================================================================
// Enums
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
pub enum JobStatus {
    #[default]
    Pending,
    Running,
    Succeeded,
    Failed,
    Completed, // Legacy: kept for backward compatibility
    DeadLetter,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "job_priority", rename_all = "snake_case")]
pub enum JobPriority {
    Critical,
    High,
    #[default]
    Normal,
    Low,
}

impl JobPriority {
    /// Convert to integer for efficient DB ordering (lower = higher priority)
    pub fn as_i16(&self) -> i16 {
        match self {
            JobPriority::Critical => 0,
            JobPriority::High => 1,
            JobPriority::Normal => 2,
            JobPriority::Low => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "overlap_policy", rename_all = "snake_case")]
pub enum OverlapPolicy {
    /// Allow multiple runs to be queued even if previous is still running
    Allow,
    /// Skip scheduling next run if a run is currently running
    #[default]
    Skip,
    /// Replace any queued run with the latest scheduled time
    CoalesceLatest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "misfire_policy", rename_all = "snake_case")]
pub enum MisfirePolicy {
    /// Enqueue all missed occurrences
    CatchUp,
    /// Only enqueue the next future occurrence
    #[default]
    SkipToLatest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "error_kind", rename_all = "snake_case")]
pub enum ErrorKind {
    /// Transient error - will retry if attempts remain
    #[default]
    Retryable,
    /// Permanent error - will not retry
    NonRetryable,
    /// Job was cancelled by user/system
    Cancelled,
    /// Job was interrupted by graceful shutdown - will retry
    Shutdown,
}

impl ErrorKind {
    /// Whether this error kind should trigger a retry
    pub fn should_retry(&self) -> bool {
        matches!(self, ErrorKind::Retryable | ErrorKind::Shutdown)
    }
}

impl From<seesaw::FailureKind> for ErrorKind {
    fn from(kind: seesaw::FailureKind) -> Self {
        match kind {
            seesaw::FailureKind::Retryable => ErrorKind::Retryable,
            seesaw::FailureKind::NonRetryable => ErrorKind::NonRetryable,
        }
    }
}

// ============================================================================
// Job Model
// ============================================================================

#[derive(FromRow, Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct Job {
    #[builder(default = db_id())]
    pub id: Uuid,

    // Core identity
    pub reference_id: Uuid,
    pub job_type: String,

    // Scheduling
    #[builder(default, setter(strip_option))]
    pub frequency: Option<String>,
    #[builder(default = "UTC".to_string())]
    pub timezone: String,
    #[builder(default, setter(strip_option))]
    pub next_run_at: Option<DateTime<Utc>>,
    #[builder(default, setter(strip_option))]
    pub last_run_at: Option<DateTime<Utc>>,

    // Payload
    #[builder(default, setter(strip_option))]
    pub args: Option<serde_json::Value>,
    #[builder(default = 1)]
    pub version: i32,

    // Policies
    #[builder(default)]
    pub priority: JobPriority,
    #[builder(default)]
    pub overlap_policy: OverlapPolicy,
    #[builder(default)]
    pub misfire_policy: MisfirePolicy,

    // Execution settings
    #[builder(default = 3)]
    pub max_retries: i32,
    #[builder(default = 0)]
    pub retry_count: i32,
    #[builder(default = 300_000)] // 5 minutes
    pub timeout_ms: i64,
    #[builder(default = 60_000)] // 1 minute
    pub lease_duration_ms: i64,

    // Lease management
    #[builder(default, setter(strip_option))]
    pub lease_expires_at: Option<DateTime<Utc>>,
    #[builder(default, setter(strip_option))]
    pub worker_id: Option<String>,

    // State
    #[builder(default)]
    pub status: JobStatus,
    #[builder(default = true)]
    pub enabled: bool,

    // Multi-tenancy
    #[builder(default, setter(strip_option))]
    pub container_id: Option<Uuid>,

    // Workflow coordination
    #[builder(default, setter(strip_option))]
    pub workflow_id: Option<Uuid>,

    // Error tracking
    #[builder(default, setter(strip_option))]
    pub error_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub error_kind: Option<ErrorKind>,

    // Dead letter workflow
    #[builder(default, setter(strip_option))]
    pub dead_lettered_at: Option<DateTime<Utc>>,
    #[builder(default, setter(strip_option))]
    pub dead_letter_reason: Option<String>,
    #[builder(default = 0)]
    pub replay_count: i32,
    #[builder(default, setter(strip_option))]
    pub resolved_at: Option<DateTime<Utc>>,
    #[builder(default, setter(strip_option))]
    pub resolution_note: Option<String>,

    // Retry chain tracing
    #[builder(default, setter(strip_option))]
    pub root_job_id: Option<Uuid>,
    #[builder(default, setter(strip_option))]
    pub dedupe_key: Option<String>,
    #[builder(default = 1)]
    pub attempt: i32,

    // Command-level idempotency
    #[builder(default, setter(strip_option))]
    pub idempotency_key: Option<String>,
    #[builder(default = 1)]
    pub command_version: i32,

    // Timestamps
    #[builder(default = Utc::now())]
    pub created_at: DateTime<Utc>,
    #[builder(default = Utc::now())]
    pub updated_at: DateTime<Utc>,
}

impl Job {
    /// Create an immediate one-time job (convenience constructor)
    pub fn immediate(reference_id: Uuid, job_type: &str) -> Self {
        Self::builder()
            .reference_id(reference_id)
            .job_type(job_type.to_string())
            .build()
    }

    /// Create a scheduled job (convenience constructor)
    pub fn scheduled(reference_id: Uuid, job_type: &str, run_at: DateTime<Utc>) -> Self {
        Self::builder()
            .reference_id(reference_id)
            .job_type(job_type.to_string())
            .next_run_at(run_at)
            .build()
    }

    /// Create a recurring job (convenience constructor)
    pub fn recurring(reference_id: Uuid, job_type: &str, frequency: &str) -> Self {
        Self::builder()
            .reference_id(reference_id)
            .job_type(job_type.to_string())
            .frequency(frequency.to_string())
            .build()
    }

    /// Legacy constructor for backward compatibility with TestJobManager
    pub fn new(
        frequency: Option<String>,
        job_type: String,
        next_run_at: Option<DateTime<Utc>>,
        timezone: Option<String>,
        container_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: db_id(),
            reference_id,
            job_type,
            frequency,
            timezone: timezone.unwrap_or_else(|| "UTC".to_string()),
            next_run_at,
            last_run_at: None,
            args: None,
            version: 1,
            priority: JobPriority::default(),
            overlap_policy: OverlapPolicy::default(),
            misfire_policy: MisfirePolicy::default(),
            max_retries: 3,
            retry_count: 0,
            timeout_ms: 300_000,
            lease_duration_ms: 60_000,
            lease_expires_at: None,
            worker_id: None,
            status: JobStatus::Pending,
            enabled: true,
            container_id,
            workflow_id: None,
            error_message: None,
            error_kind: None,
            dead_lettered_at: None,
            dead_letter_reason: None,
            replay_count: 0,
            resolved_at: None,
            resolution_note: None,
            root_job_id: None,
            dedupe_key: None,
            attempt: 1,
            idempotency_key: None,
            command_version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Create a job for a serialized command.
    ///
    /// This constructor is used by `JobQueue` to create jobs from Commands.
    pub fn for_command(
        job_type: &str,
        args: serde_json::Value,
        run_at: Option<DateTime<Utc>>,
        idempotency_key: Option<String>,
        command_version: i32,
        priority: JobPriority,
        max_retries: i32,
        container_id: Option<Uuid>,
        lease_duration_ms: i64,
    ) -> Self {
        Self {
            id: db_id(),
            job_type: job_type.to_string(),
            frequency: None,
            timezone: "UTC".to_string(),
            next_run_at: run_at,
            last_run_at: None,
            args: Some(args),
            version: 1,
            priority,
            overlap_policy: OverlapPolicy::default(),
            misfire_policy: MisfirePolicy::default(),
            max_retries,
            retry_count: 0,
            timeout_ms: 300_000,
            lease_duration_ms,
            lease_expires_at: None,
            worker_id: None,
            status: JobStatus::Pending,
            enabled: true,
            container_id,
            workflow_id: None,
            error_message: None,
            error_kind: None,
            dead_lettered_at: None,
            dead_letter_reason: None,
            replay_count: 0,
            resolved_at: None,
            resolution_note: None,
            root_job_id: None,
            dedupe_key: None,
            attempt: 1,
            idempotency_key,
            command_version,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Check if this is a recurring job
    pub fn is_recurring(&self) -> bool {
        self.frequency.is_some()
    }

    /// Find a job by reference_id and job_type
    pub async fn find_by_reference(
        job_type: &str,
        kernel: &ServerKernel,
    ) -> Result<Option<Self>> {
        let job = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                   max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                   args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                   lease_expires_at, worker_id, enabled, error_message, error_kind,
                   dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                   root_job_id, dedupe_key, attempt, idempotency_key, command_version
            FROM jobs
            WHERE reference_id = $1 AND job_type = $2
            LIMIT 1
            "#,
        )
        .bind(reference_id)
        .bind(job_type)
        .fetch_optional(&kernel.db_connection)
        .await?;

        Ok(job)
    }

    /// Delete a job by reference_id and job_type
    pub async fn delete_by_reference(
        job_type: &str,
        kernel: &ServerKernel,
    ) -> Result<u64> {
        let deleted_count =
            sqlx::query("DELETE FROM jobs WHERE reference_id = $1 AND job_type = $2")
                .bind(reference_id)
                .bind(job_type)
                .execute(&kernel.db_connection)
                .await?
                .rows_affected();

        Ok(deleted_count)
    }

    /// Check if the job is ready to run
    pub fn is_ready(&self) -> bool {
        if self.status != JobStatus::Pending {
            return false;
        }

        if !self.enabled {
            return false;
        }

        if self.retry_count >= self.max_retries {
            return false;
        }

        match self.next_run_at {
            None => true,
            Some(next_run) => next_run <= Utc::now(),
        }
    }

    /// Calculate the next run time from frequency
    pub fn calculate_next_run_at_from_frequency(&self) -> Result<Option<DateTime<Utc>>> {
        match &self.frequency {
            None => Ok(None),
            Some(freq) => {
                if is_rrule(freq) {
                    calculate_next_run_at(freq, &self.timezone)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Find the next scheduled run time for any pending job
    pub async fn find_next_run_time(kernel: &ServerKernel) -> Result<Option<DateTime<Utc>>> {
        let result = sqlx::query_scalar::<_, DateTime<Utc>>(
            r#"
            SELECT next_run_at
            FROM jobs
            WHERE status = 'pending'
              AND enabled = true
              AND next_run_at IS NOT NULL
              AND retry_count < max_retries
            ORDER BY next_run_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&kernel.db_connection)
        .await?;

        Ok(result)
    }

    /// Find jobs that are ready to run (for polling)
    pub async fn find_ready_jobs(limit: i64, kernel: &ServerKernel) -> Result<Vec<Self>> {
        let jobs = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, status, frequency, reference_id, job_type, timezone,
                   last_run_at, next_run_at, max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                   args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                   lease_expires_at, worker_id, enabled, error_message, error_kind,
                   dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                   root_job_id, dedupe_key, attempt, idempotency_key, command_version
            FROM jobs
            WHERE status = 'pending'
              AND enabled = true
              AND (next_run_at IS NULL OR next_run_at <= NOW())
              AND retry_count < max_retries
            ORDER BY priority, COALESCE(next_run_at, created_at) ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&kernel.db_connection)
        .await?;

        Ok(jobs)
    }

    /// Claim jobs atomically using FOR UPDATE SKIP LOCKED
    /// Also recovers stale jobs with expired leases
    pub async fn claim_jobs(
        limit: i64,
        worker_id: &str,
        lease_duration_ms: i64,
        kernel: &ServerKernel,
    ) -> Result<Vec<Self>> {
        let jobs = sqlx::query_as::<_, Self>(
            r#"
            WITH next_jobs AS (
                SELECT id
                FROM jobs
                WHERE
                    (status = 'pending' AND enabled = true AND (next_run_at IS NULL OR next_run_at <= NOW()) AND retry_count < max_retries)
                    OR (status = 'running' AND lease_expires_at < NOW())
                ORDER BY priority, COALESCE(next_run_at, created_at)
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            UPDATE jobs
            SET
                status = 'running',
                last_run_at = COALESCE(last_run_at, NOW()),
                lease_expires_at = NOW() + ($2 || ' milliseconds')::INTERVAL,
                worker_id = $3,
                updated_at = NOW()
            WHERE id IN (SELECT id FROM next_jobs)
            RETURNING id, status, frequency, reference_id, job_type, timezone,
                      last_run_at, next_run_at, max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                      args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                      lease_expires_at, worker_id, enabled, error_message, error_kind,
                      dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                      root_job_id, dedupe_key, attempt, idempotency_key, command_version
            "#,
        )
        .bind(limit)
        .bind(lease_duration_ms.to_string())
        .bind(worker_id)
        .fetch_all(&kernel.db_connection)
        .await?;

        Ok(jobs)
    }

    /// Extend the lease for a running job (heartbeat)
    pub async fn extend_lease(&self, kernel: &ServerKernel) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET lease_expires_at = NOW() + ($1 || ' milliseconds')::INTERVAL,
                updated_at = NOW()
            WHERE id = $2 AND status = 'running'
            "#,
        )
        .bind(self.lease_duration_ms.to_string())
        .bind(self.id)
        .execute(&kernel.db_connection)
        .await?;

        Ok(())
    }

    /// Upsert a job by reference_id and job_type
    pub async fn upsert(self, kernel: &ServerKernel) -> Result<Self> {
        match Self::find_by_reference(self.reference_id, &self.job_type, kernel).await? {
            Some(existing) => {
                let mut updated = self;
                updated.id = existing.id;
                updated.created_at = existing.created_at;
                updated.version = existing.version + 1;
                updated.update(kernel).await
            }
            None => self.insert(kernel).await,
        }
    }

    /// Create a retry job from a failed job
    pub fn create_retry(&self, scheduled_for: DateTime<Utc>) -> Self {
        Self {
            id: db_id(),
            job_type: self.job_type.clone(),
            frequency: self.frequency.clone(),
            timezone: self.timezone.clone(),
            next_run_at: Some(scheduled_for),
            last_run_at: None,
            args: self.args.clone(),
            version: self.version,
            priority: self.priority,
            overlap_policy: self.overlap_policy,
            misfire_policy: self.misfire_policy,
            max_retries: self.max_retries,
            retry_count: self.retry_count + 1,
            timeout_ms: self.timeout_ms,
            lease_duration_ms: self.lease_duration_ms,
            lease_expires_at: None,
            worker_id: None,
            status: JobStatus::Pending,
            enabled: true,
            container_id: self.container_id,
            workflow_id: self.workflow_id,
            error_message: None,
            error_kind: None,
            dead_lettered_at: None,
            dead_letter_reason: None,
            replay_count: 0,
            resolved_at: None,
            resolution_note: None,
            root_job_id: self.root_job_id.or(Some(self.id)),
            dedupe_key: self.dedupe_key.clone(),
            attempt: self.attempt + 1,
            idempotency_key: self.idempotency_key.clone(),
            command_version: self.command_version,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Mark job as dead letter
    pub async fn mark_dead_letter(&mut self, reason: &str, kernel: &ServerKernel) -> Result<()> {
        self.status = JobStatus::DeadLetter;
        self.dead_lettered_at = Some(Utc::now());
        self.dead_letter_reason = Some(reason.to_string());
        self.update(kernel).await?;
        Ok(())
    }

    /// Count pending jobs for a workflow
    pub async fn count_pending_for_workflow(workflow_id: Uuid, db: &sqlx::PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM jobs
            WHERE workflow_id = $1 AND status = 'pending'
            "#,
        )
        .bind(workflow_id)
        .fetch_one(db)
        .await?;

        Ok(count)
    }

    /// Count running jobs for a workflow
    pub async fn count_running_for_workflow(workflow_id: Uuid, db: &sqlx::PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM jobs
            WHERE workflow_id = $1 AND status = 'running'
            "#,
        )
        .bind(workflow_id)
        .fetch_one(db)
        .await?;

        Ok(count)
    }

    /// Find jobs by workflow ID
    pub async fn find_by_workflow(
        workflow_id: Uuid,
        limit: i64,
        db: &sqlx::PgPool,
    ) -> Result<Vec<Self>> {
        let jobs = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                   max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                   args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                   lease_expires_at, worker_id, enabled, error_message, error_kind,
                   dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                   root_job_id, dedupe_key, attempt, idempotency_key, command_version
            FROM jobs
            WHERE workflow_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(workflow_id)
        .bind(limit)
        .fetch_all(db)
        .await?;

        Ok(jobs)
    }

    /// Insert with a PgPool directly (for use by SeesawJobQueueAdapter).
    ///
    /// This bypasses the Record trait to allow inserting without a full ServerKernel.
    pub async fn insert_with_pool(&self, pool: &sqlx::PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO jobs (
                id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                lease_expires_at, worker_id, enabled, error_message, error_kind,
                dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                root_job_id, dedupe_key, attempt, idempotency_key, command_version
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21,
                $22, $23, $24, $25, $26,
                $27, $28, $29, $30, $31,
                $32, $33, $34, $35, $36
            )
            ON CONFLICT (reference_id, job_type) DO UPDATE SET
                status = EXCLUDED.status,
                frequency = EXCLUDED.frequency,
                timezone = EXCLUDED.timezone,
                next_run_at = EXCLUDED.next_run_at,
                args = EXCLUDED.args,
                version = jobs.version + 1,
                priority = EXCLUDED.priority,
                overlap_policy = EXCLUDED.overlap_policy,
                misfire_policy = EXCLUDED.misfire_policy,
                max_retries = EXCLUDED.max_retries,
                timeout_ms = EXCLUDED.timeout_ms,
                lease_duration_ms = EXCLUDED.lease_duration_ms,
                container_id = EXCLUDED.container_id,
                workflow_id = EXCLUDED.workflow_id,
                enabled = EXCLUDED.enabled,
                updated_at = NOW()
            RETURNING id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                      max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                      args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                      lease_expires_at, worker_id, enabled, error_message, error_kind,
                      dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                      root_job_id, dedupe_key, attempt, idempotency_key, command_version
            "#,
        )
        .bind(self.id)
        .bind(self.status)
        .bind(&self.frequency)
        .bind(self.reference_id)
        .bind(&self.job_type)
        .bind(&self.timezone)
        .bind(self.last_run_at)
        .bind(self.next_run_at)
        .bind(self.max_retries)
        .bind(self.retry_count)
        .bind(self.created_at)
        .bind(self.updated_at)
        .bind(self.container_id)
        .bind(self.workflow_id)
        .bind(&self.args)
        .bind(self.version)
        .bind(self.priority)
        .bind(self.overlap_policy)
        .bind(self.misfire_policy)
        .bind(self.timeout_ms)
        .bind(self.lease_duration_ms)
        .bind(self.lease_expires_at)
        .bind(&self.worker_id)
        .bind(self.enabled)
        .bind(&self.error_message)
        .bind(self.error_kind)
        .bind(self.dead_lettered_at)
        .bind(&self.dead_letter_reason)
        .bind(self.replay_count)
        .bind(self.resolved_at)
        .bind(&self.resolution_note)
        .bind(self.root_job_id)
        .bind(&self.dedupe_key)
        .bind(self.attempt)
        .bind(&self.idempotency_key)
        .bind(self.command_version)
        .fetch_one(pool)
        .await?;

        Ok(job)
    }
}

#[async_trait::async_trait]
impl Record for Job {
    const TABLE: &'static str = "jobs";
    type Id = uuid::Uuid;

    async fn find_by_id(id: Uuid, db: &sqlx::PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                   max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                   args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                   lease_expires_at, worker_id, enabled, error_message, error_kind,
                   dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                   root_job_id, dedupe_key, attempt, idempotency_key, command_version
            FROM jobs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(db)
        .await?;

        Ok(job)
    }

    async fn insert(&self, kernel: &ServerKernel) -> Result<Self> {
        let job = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO jobs (
                id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                lease_expires_at, worker_id, enabled, error_message, error_kind,
                dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                root_job_id, dedupe_key, attempt, idempotency_key, command_version
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21,
                $22, $23, $24, $25, $26,
                $27, $28, $29, $30, $31,
                $32, $33, $34, $35, $36
            )
            ON CONFLICT (reference_id, job_type) DO UPDATE SET
                status = EXCLUDED.status,
                frequency = EXCLUDED.frequency,
                timezone = EXCLUDED.timezone,
                next_run_at = EXCLUDED.next_run_at,
                args = EXCLUDED.args,
                version = jobs.version + 1,
                priority = EXCLUDED.priority,
                overlap_policy = EXCLUDED.overlap_policy,
                misfire_policy = EXCLUDED.misfire_policy,
                max_retries = EXCLUDED.max_retries,
                timeout_ms = EXCLUDED.timeout_ms,
                lease_duration_ms = EXCLUDED.lease_duration_ms,
                container_id = EXCLUDED.container_id,
                workflow_id = EXCLUDED.workflow_id,
                enabled = EXCLUDED.enabled,
                updated_at = NOW()
            RETURNING id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                      max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                      args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                      lease_expires_at, worker_id, enabled, error_message, error_kind,
                      dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                      root_job_id, dedupe_key, attempt, idempotency_key, command_version
            "#,
        )
        .bind(self.id)
        .bind(self.status)
        .bind(&self.frequency)
        .bind(self.reference_id)
        .bind(&self.job_type)
        .bind(&self.timezone)
        .bind(self.last_run_at)
        .bind(self.next_run_at)
        .bind(self.max_retries)
        .bind(self.retry_count)
        .bind(self.created_at)
        .bind(self.updated_at)
        .bind(self.container_id)
        .bind(self.workflow_id)
        .bind(&self.args)
        .bind(self.version)
        .bind(self.priority)
        .bind(self.overlap_policy)
        .bind(self.misfire_policy)
        .bind(self.timeout_ms)
        .bind(self.lease_duration_ms)
        .bind(self.lease_expires_at)
        .bind(&self.worker_id)
        .bind(self.enabled)
        .bind(&self.error_message)
        .bind(self.error_kind)
        .bind(self.dead_lettered_at)
        .bind(&self.dead_letter_reason)
        .bind(self.replay_count)
        .bind(self.resolved_at)
        .bind(&self.resolution_note)
        .bind(self.root_job_id)
        .bind(&self.dedupe_key)
        .bind(self.attempt)
        .bind(&self.idempotency_key)
        .bind(self.command_version)
        .fetch_one(&kernel.db_connection)
        .await?;

        Ok(job)
    }

    async fn update(&self, kernel: &ServerKernel) -> Result<Self> {
        let job = sqlx::query_as::<_, Self>(
            r#"
            UPDATE jobs SET
                status = $1, frequency = $2, reference_id = $3, job_type = $4, timezone = $5,
                last_run_at = $6, next_run_at = $7, max_retries = $8, retry_count = $9,
                container_id = $10, workflow_id = $11, args = $12, version = $13, priority = $14,
                overlap_policy = $15, misfire_policy = $16, timeout_ms = $17, lease_duration_ms = $18,
                lease_expires_at = $19, worker_id = $20, enabled = $21, error_message = $22, error_kind = $23,
                dead_lettered_at = $24, dead_letter_reason = $25, replay_count = $26, resolved_at = $27, resolution_note = $28,
                root_job_id = $29, dedupe_key = $30, attempt = $31, idempotency_key = $32, command_version = $33,
                updated_at = NOW()
            WHERE id = $34
            RETURNING id, status, frequency, reference_id, job_type, timezone, last_run_at, next_run_at,
                      max_retries, retry_count, created_at, updated_at, container_id, workflow_id,
                      args, version, priority, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms,
                      lease_expires_at, worker_id, enabled, error_message, error_kind,
                      dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note,
                      root_job_id, dedupe_key, attempt, idempotency_key, command_version
            "#,
        )
        .bind(self.status)
        .bind(&self.frequency)
        .bind(self.reference_id)
        .bind(&self.job_type)
        .bind(&self.timezone)
        .bind(self.last_run_at)
        .bind(self.next_run_at)
        .bind(self.max_retries)
        .bind(self.retry_count)
        .bind(self.container_id)
        .bind(self.workflow_id)
        .bind(&self.args)
        .bind(self.version)
        .bind(self.priority)
        .bind(self.overlap_policy)
        .bind(self.misfire_policy)
        .bind(self.timeout_ms)
        .bind(self.lease_duration_ms)
        .bind(self.lease_expires_at)
        .bind(&self.worker_id)
        .bind(self.enabled)
        .bind(&self.error_message)
        .bind(self.error_kind)
        .bind(self.dead_lettered_at)
        .bind(&self.dead_letter_reason)
        .bind(self.replay_count)
        .bind(self.resolved_at)
        .bind(&self.resolution_note)
        .bind(self.root_job_id)
        .bind(&self.dedupe_key)
        .bind(self.attempt)
        .bind(&self.idempotency_key)
        .bind(self.command_version)
        .bind(self.id)
        .fetch_one(&kernel.db_connection)
        .await?;

        Ok(job)
    }

    async fn delete(&self, kernel: &ServerKernel) -> Result<()> {
        sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(self.id)
            .execute(&kernel.db_connection)
            .await?;

        Ok(())
    }

    async fn read(&self, kernel: &ServerKernel) -> Result<Self> {
        Self::find_by_id(self.id, &kernel.db_connection).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_job() -> Job {
        Job::immediate(Uuid::new_v4(), "test_job")
    }

    #[test]
    fn new_job_has_default_max_retries_of_3() {
        let job = sample_job();
        assert_eq!(job.max_retries, 3);
    }

    #[test]
    fn new_job_has_retry_count_of_0() {
        let job = sample_job();
        assert_eq!(job.retry_count, 0);
    }

    #[test]
    fn new_job_defaults_to_utc_timezone() {
        let job = sample_job();
        assert_eq!(job.timezone, "UTC");
    }

    #[test]
    fn new_job_starts_with_pending_status() {
        let job = sample_job();
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn new_job_has_normal_priority_by_default() {
        let job = sample_job();
        assert_eq!(job.priority, JobPriority::Normal);
    }

    #[test]
    fn is_ready_pending_job_without_schedule() {
        let job = sample_job();
        assert!(job.is_ready());
    }

    #[test]
    fn is_ready_disabled_job_is_not_ready() {
        let mut job = sample_job();
        job.enabled = false;
        assert!(!job.is_ready());
    }

    #[test]
    fn is_ready_running_job_is_not_ready() {
        let mut job = sample_job();
        job.status = JobStatus::Running;
        assert!(!job.is_ready());
    }

    #[test]
    fn retryable_error_should_retry() {
        assert!(ErrorKind::Retryable.should_retry());
        assert!(ErrorKind::Shutdown.should_retry());
    }

    #[test]
    fn non_retryable_error_should_not_retry() {
        assert!(!ErrorKind::NonRetryable.should_retry());
        assert!(!ErrorKind::Cancelled.should_retry());
    }

    #[test]
    fn priority_ordering_is_correct() {
        assert!(JobPriority::Critical.as_i16() < JobPriority::High.as_i16());
        assert!(JobPriority::High.as_i16() < JobPriority::Normal.as_i16());
        assert!(JobPriority::Normal.as_i16() < JobPriority::Low.as_i16());
    }
}
