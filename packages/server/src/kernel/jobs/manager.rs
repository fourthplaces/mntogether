//! Job manager trait and implementations.
//!
//! The `JobManager` trait abstracts job scheduling and execution,
//! allowing different implementations for production and testing.

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use super::Job;

/// Options for scheduling a job.
#[derive(Clone, Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct ScheduleOptions {
    /// The ID of the entity this job references.
    pub reference_id: Uuid,
    /// The type of job (must match a registered handler).
    pub job_type: String,
    /// Optional RRULE for recurring jobs.
    #[builder(default)]
    pub frequency: Option<String>,
    /// When to run the job. If None, runs immediately.
    #[builder(default)]
    pub run_at: Option<DateTime<Utc>>,
    /// Timezone for RRULE calculations.
    #[builder(default)]
    pub timezone: Option<String>,
    /// Maximum retries (default: 2).
    #[builder(default)]
    pub max_retries: Option<i32>,
    /// Optional container scope for the job.
    #[builder(default)]
    pub container_id: Option<Uuid>,
}

impl ScheduleOptions {
    /// Create options for a one-time immediate job.
    pub fn immediate(reference_id: Uuid, job_type: impl Into<String>) -> Self {
        Self::builder()
            .reference_id(reference_id)
            .job_type(job_type)
            .build()
    }

    /// Create options for a scheduled job.
    pub fn scheduled(
        reference_id: Uuid,
        job_type: impl Into<String>,
        run_at: DateTime<Utc>,
    ) -> Self {
        Self::builder()
            .reference_id(reference_id)
            .job_type(job_type)
            .run_at(Some(run_at))
            .build()
    }

    /// Create options for a recurring job.
    pub fn recurring(
        reference_id: Uuid,
        job_type: impl Into<String>,
        rrule: impl Into<String>,
        timezone: Option<String>,
    ) -> Self {
        Self::builder()
            .reference_id(reference_id)
            .job_type(job_type)
            .frequency(Some(rrule.into()))
            .timezone(timezone)
            .build()
    }
}

/// Trait for managing job scheduling and execution.
///
/// Implementations handle where jobs are stored and how they're executed.
/// - Production: Uses database + seesaw effects for execution
/// - Testing: In-memory storage for inspection
#[async_trait]
pub trait JobManager: Send + Sync {
    /// Schedule a job for execution.
    async fn schedule(&self, options: ScheduleOptions) -> Result<Job>;

    /// Cancel a scheduled job.
    async fn cancel(&self, reference_id: Uuid, job_type: &str) -> Result<bool>;
}

/// Test job manager that stores jobs in memory.
///
/// Jobs are stored in memory for inspection. Actual execution
/// is handled by seesaw effects.
pub struct TestJobManager {
    jobs: RwLock<HashMap<Uuid, Job>>,
    /// Track which jobs have been executed
    executed: RwLock<Vec<Uuid>>,
}

impl Default for TestJobManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TestJobManager {
    /// Create a new test job manager.
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
            executed: RwLock::new(Vec::new()),
        }
    }

    /// Get all scheduled jobs.
    pub fn jobs(&self) -> Vec<Job> {
        self.jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Get jobs by type.
    pub fn jobs_by_type(&self, job_type: &str) -> Vec<Job> {
        self.jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|j| j.job_type == job_type)
            .cloned()
            .collect()
    }

    /// Get a job by reference ID and type.
    pub fn get_job(&self, reference_id: Uuid, job_type: &str) -> Option<Job> {
        self.jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .find(|j| j.reference_id == reference_id && j.job_type == job_type)
            .cloned()
    }

    /// Check if a job was scheduled.
    pub fn was_scheduled(&self, reference_id: Uuid, job_type: &str) -> bool {
        self.get_job(reference_id, job_type).is_some()
    }

    /// Check if a job was executed.
    pub fn was_executed(&self, job_id: Uuid) -> bool {
        self.executed
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&job_id)
    }

    /// Get all executed job IDs.
    pub fn executed_jobs(&self) -> Vec<Uuid> {
        self.executed
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Mark a job as executed.
    pub fn mark_executed(&self, job_id: Uuid) {
        self.executed
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(job_id);
    }

    /// Clear all jobs and execution history.
    pub fn clear(&self) {
        self.jobs.write().unwrap_or_else(|e| e.into_inner()).clear();
        self.executed
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

#[async_trait]
impl JobManager for TestJobManager {
    async fn schedule(&self, options: ScheduleOptions) -> Result<Job> {
        // Create job
        let mut job = Job::new(
            options.frequency,
            options.reference_id,
            options.job_type,
            options.run_at,
            options.timezone,
            options.container_id,
        );

        if let Some(max_retries) = options.max_retries {
            job.max_retries = max_retries;
        }

        // Store in memory
        self.jobs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(job.id, job.clone());

        Ok(job)
    }

    async fn cancel(&self, reference_id: Uuid, job_type: &str) -> Result<bool> {
        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());

        // Find and remove the job
        let job_id = jobs
            .iter()
            .find(|(_, j)| j.reference_id == reference_id && j.job_type == job_type)
            .map(|(id, _)| *id);

        if let Some(id) = job_id {
            jobs.remove(&id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// A mock job handler for testing.
///
/// Records all job invocations for later inspection.
pub struct MockJobHandler {
    job_type: &'static str,
    invocations: RwLock<Vec<Job>>,
    should_fail: RwLock<bool>,
}

impl MockJobHandler {
    /// Create a new mock handler.
    pub fn new(job_type: &'static str) -> Self {
        Self {
            job_type,
            invocations: RwLock::new(Vec::new()),
            should_fail: RwLock::new(false),
        }
    }

    /// Get all invocations.
    pub fn invocations(&self) -> Vec<Job> {
        self.invocations
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Get invocation count.
    pub fn invocation_count(&self) -> usize {
        self.invocations
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Check if invoked with a specific reference ID.
    pub fn was_invoked_with(&self, reference_id: Uuid) -> bool {
        self.invocations
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|j| j.reference_id == reference_id)
    }

    /// Set whether the handler should fail.
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.write().unwrap_or_else(|e| e.into_inner()) = should_fail;
    }

    /// Clear invocations.
    pub fn clear(&self) {
        self.invocations
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Record an invocation.
    pub fn record_invocation(&self, job: &Job) {
        self.invocations
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(job.clone());
    }

    /// Get job type.
    pub fn job_type(&self) -> &'static str {
        self.job_type
    }

    /// Check if should fail.
    pub fn should_fail(&self) -> bool {
        *self.should_fail.read().unwrap_or_else(|e| e.into_inner())
    }
}

/// Default job manager for production.
///
/// Job execution is handled by seesaw effects.
pub struct DefaultJobManager;

impl DefaultJobManager {
    /// Create a new default job manager.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultJobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobManager for DefaultJobManager {
    async fn schedule(&self, options: ScheduleOptions) -> Result<Job> {
        // Create job (persistence is handled by effects in production)
        let mut job = Job::new(
            options.frequency,
            options.reference_id,
            options.job_type,
            options.run_at,
            options.timezone,
            options.container_id,
        );

        if let Some(max_retries) = options.max_retries {
            job.max_retries = max_retries;
        }

        Ok(job)
    }

    async fn cancel(&self, _reference_id: Uuid, _job_type: &str) -> Result<bool> {
        // In production, cancellation is handled by deleting from DB
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_options_immediate() {
        let id = Uuid::new_v4();
        let opts = ScheduleOptions::immediate(id, "test");

        assert_eq!(opts.reference_id, id);
        assert_eq!(opts.job_type, "test");
        assert!(opts.run_at.is_none());
        assert!(opts.frequency.is_none());
    }

    #[test]
    fn schedule_options_scheduled() {
        let id = Uuid::new_v4();
        let run_at = Utc::now();
        let opts = ScheduleOptions::scheduled(id, "test", run_at);

        assert_eq!(opts.run_at, Some(run_at));
    }

    #[test]
    fn schedule_options_recurring() {
        let id = Uuid::new_v4();
        let opts = ScheduleOptions::recurring(id, "test", "FREQ=DAILY", Some("UTC".into()));

        assert_eq!(opts.frequency, Some("FREQ=DAILY".into()));
        assert_eq!(opts.timezone, Some("UTC".into()));
    }

    #[tokio::test]
    async fn test_job_manager_schedule() {
        let manager = TestJobManager::new();

        let job = manager
            .schedule(ScheduleOptions::immediate(Uuid::new_v4(), "test_job"))
            .await
            .unwrap();

        assert!(manager.was_scheduled(job.reference_id, "test_job"));
        assert_eq!(manager.jobs().len(), 1);
    }

    #[tokio::test]
    async fn test_job_manager_cancel() {
        let manager = TestJobManager::new();

        let ref_id = Uuid::new_v4();
        manager
            .schedule(ScheduleOptions::immediate(ref_id, "test_job"))
            .await
            .unwrap();

        assert!(manager.cancel(ref_id, "test_job").await.unwrap());
        assert!(!manager.was_scheduled(ref_id, "test_job"));
    }

    #[test]
    fn mock_handler_tracks_invocations() {
        let handler = MockJobHandler::new("test");
        assert_eq!(handler.invocation_count(), 0);
    }
}
