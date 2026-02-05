//! Job testing utilities.
//!
//! Re-exports and utilities for testing job execution.

pub use super::manager::{MockJobHandler, ScheduleOptions, TestJobManager};

use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::job::ErrorKind;
use super::queue::{ClaimedJob, EnqueueCommand, EnqueueResult, JobQueue};
use super::Job;
use crate::kernel::ServerKernel;

/// Process all ready jobs from the database.
///
/// Finds all pending jobs that are ready to run.
/// Returns the jobs found (execution is handled by the job engine).
pub async fn find_ready_jobs_for_testing(kernel: &ServerKernel) -> Result<Vec<Job>> {
    let ready_jobs = Job::find_ready_jobs(100, kernel).await?;
    Ok(ready_jobs)
}

// =============================================================================
// NoopJobQueue - Does nothing (for production until worker is running)
// =============================================================================

/// A no-op job queue that accepts commands but doesn't persist them.
/// Use this when job execution is not needed (e.g., during development).
#[derive(Debug, Clone, Default)]
pub struct NoopJobQueue;

impl NoopJobQueue {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl JobQueue for NoopJobQueue {
    async fn enqueue_command(&self, _command: EnqueueCommand) -> Result<EnqueueResult> {
        Ok(EnqueueResult::Created(Uuid::new_v4()))
    }

    async fn schedule_command(
        &self,
        _command: EnqueueCommand,
        _run_at: DateTime<Utc>,
    ) -> Result<EnqueueResult> {
        Ok(EnqueueResult::Created(Uuid::new_v4()))
    }

    async fn claim(&self, _worker_id: &str, _limit: i64) -> Result<Vec<ClaimedJob>> {
        Ok(vec![])
    }

    async fn mark_succeeded(&self, _job_id: Uuid) -> Result<()> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: Uuid, _error: &str, _kind: ErrorKind) -> Result<()> {
        Ok(())
    }

    async fn cancel(&self, _job_id: Uuid) -> Result<bool> {
        Ok(true)
    }

    async fn heartbeat(&self, _job_id: Uuid) -> Result<()> {
        Ok(())
    }

    async fn next_run_time(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(None)
    }
}

// =============================================================================
// SpyJobQueue - Records enqueued commands for testing
// =============================================================================

/// A recorded command from the spy job queue.
#[derive(Debug, Clone)]
pub struct RecordedCommand {
    pub command_type: String,
    pub payload: serde_json::Value,
    pub run_at: Option<DateTime<Utc>>,
}

/// A spy job queue that records all enqueued commands for testing.
#[derive(Debug, Clone, Default)]
pub struct SpyJobQueue {
    commands: Arc<Mutex<Vec<RecordedCommand>>>,
}

impl SpyJobQueue {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all recorded commands
    pub fn recorded_commands(&self) -> Vec<RecordedCommand> {
        self.commands.lock().unwrap().clone()
    }

    /// Clear all recorded commands
    pub fn clear(&self) {
        self.commands.lock().unwrap().clear();
    }

    /// Check if a command of the given type was enqueued
    pub fn has_command(&self, command_type: &str) -> bool {
        self.commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.command_type == command_type)
    }
}

#[async_trait]
impl JobQueue for SpyJobQueue {
    async fn enqueue_command(&self, command: EnqueueCommand) -> Result<EnqueueResult> {
        let recorded = RecordedCommand {
            command_type: command.command_type.to_string(),
            payload: command.payload,
            run_at: None,
        };
        self.commands.lock().unwrap().push(recorded);
        Ok(EnqueueResult::Created(Uuid::new_v4()))
    }

    async fn schedule_command(
        &self,
        command: EnqueueCommand,
        run_at: DateTime<Utc>,
    ) -> Result<EnqueueResult> {
        let recorded = RecordedCommand {
            command_type: command.command_type.to_string(),
            payload: command.payload,
            run_at: Some(run_at),
        };
        self.commands.lock().unwrap().push(recorded);
        Ok(EnqueueResult::Created(Uuid::new_v4()))
    }

    async fn claim(&self, _worker_id: &str, _limit: i64) -> Result<Vec<ClaimedJob>> {
        Ok(vec![])
    }

    async fn mark_succeeded(&self, _job_id: Uuid) -> Result<()> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: Uuid, _error: &str, _kind: ErrorKind) -> Result<()> {
        Ok(())
    }

    async fn cancel(&self, _job_id: Uuid) -> Result<bool> {
        Ok(true)
    }

    async fn heartbeat(&self, _job_id: Uuid) -> Result<()> {
        Ok(())
    }

    async fn next_run_time(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(None)
    }
}
