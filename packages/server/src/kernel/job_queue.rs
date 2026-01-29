//! Job queue adapter for seesaw-rs using PostgreSQL storage.
//!
//! This module provides a bridge between seesaw's job queue abstraction and
//! PostgreSQL-based job storage.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use seesaw::JobSpec;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;

/// PostgreSQL-backed job for background processing
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub status: String,
    pub job_type: String,
    pub args: serde_json::Value,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub max_retries: i32,
    pub retry_count: i32,
    pub version: i32,
    pub idempotency_key: Option<String>,
    pub reference_id: Option<Uuid>,
    pub priority: i32,
    pub error_message: Option<String>,
    pub error_kind: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Job {
    /// Create a new job for immediate execution
    pub fn new(
        job_type: String,
        args: serde_json::Value,
        reference_id: Option<Uuid>,
        idempotency_key: Option<String>,
        version: i32,
        max_retries: i32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            status: "pending".to_string(),
            job_type,
            args,
            next_run_at: Some(now),
            last_run_at: None,
            max_retries,
            retry_count: 0,
            version,
            idempotency_key,
            reference_id,
            priority: 0,
            error_message: None,
            error_kind: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Insert the job into the database
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, Job>(
            r#"
            INSERT INTO jobs (
                id, status, job_type, args, next_run_at, last_run_at,
                max_retries, retry_count, version, idempotency_key,
                reference_id, priority, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(self.id)
        .bind(&self.status)
        .bind(&self.job_type)
        .bind(&self.args)
        .bind(self.next_run_at)
        .bind(self.last_run_at)
        .bind(self.max_retries)
        .bind(self.retry_count)
        .bind(self.version)
        .bind(&self.idempotency_key)
        .bind(self.reference_id)
        .bind(self.priority)
        .bind(self.created_at)
        .bind(self.updated_at)
        .fetch_one(pool)
        .await?;

        Ok(job)
    }
}

/// Adapter that implements seesaw's `JobQueue` trait using PostgreSQL.
pub struct SeesawJobQueueAdapter {
    db: PgPool,
}

impl SeesawJobQueueAdapter {
    /// Create a new adapter using the provided database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Check if a job with the given idempotency key already exists.
    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Job>> {
        let job = sqlx::query_as::<_, Job>(
            r#"
            SELECT *
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
                    "Found existing job with idempotency key"
                );
                return Ok(existing.id);
            }
        }

        // Create job
        let mut job = Job::new(
            spec.job_type.to_string(),
            payload,
            spec.reference_id,
            spec.idempotency_key,
            spec.version,
            spec.max_retries,
        );

        // Set run_at if scheduling for later
        if let Some(run_at) = run_at {
            job.next_run_at = Some(run_at);
        }

        job.priority = spec.priority;

        debug!(
            job_id = %job.id,
            job_type = %spec.job_type,
            run_at = ?run_at,
            "Enqueueing job via seesaw adapter"
        );

        // Insert into database
        let inserted = job.insert(&self.db).await?;

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
