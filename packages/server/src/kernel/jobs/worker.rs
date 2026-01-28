//! Job worker service for processing background commands.
//!
//! The `JobWorker` is a long-running service that:
//! - Polls the database for ready jobs via `JobStore`
//! - Deserializes job payloads to commands via `CommandRegistry`
//! - Dispatches commands to effects via the seesaw `Dispatcher`
//! - Handles retries and dead-lettering
//! - Provides heartbeats for long-running jobs
//!
//! # Architecture
//!
//! ```text
//! JobWorker
//!     │
//!     ├─► Poll DB (claim jobs via JobStore)
//!     ├─► Deserialize command (CommandRegistry)
//!     ├─► Dispatch to effect (Dispatcher.dispatch)
//!     │       └─► Effect.execute(cmd, EffectContext)
//!     └─► Mark succeeded/failed via JobStore
//! ```
//!
//! Note: The worker uses `dispatch()` not `dispatch_one()` because `dispatch_one()`
//! checks `execution_mode()` and would re-enqueue background commands. Since the
//! worker is executing already-queued commands, it must use `dispatch()` for
//! inline execution regardless of execution mode.
//!
//! # Example
//!
//! ```ignore
//! use api_core::kernel::jobs::{JobWorker, PostgresJobStore};
//! use seesaw::{CommandRegistry, Dispatcher};
//!
//! // Create registry with command deserializers
//! let mut registry = CommandRegistry::new();
//! registry.register::<SendEmailCommand>("email:send", vec![1]);
//!
//! // Create worker
//! let worker = JobWorker::new(
//!     Arc::new(PostgresJobStore::new(kernel.clone())),
//!     Arc::new(registry),
//!     dispatcher,
//! );
//!
//! // Run as service
//! ServiceHost::new()
//!     .with_service(worker)
//!     .run_until_shutdown()
//!     .await;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use seesaw::CommandRegistry;
use seesaw::job::{ClaimedJob, DeserializationError, FailureKind, JobStore};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::kernel::service_host::Service;

/// Configuration for the job worker.
#[derive(Debug, Clone)]
pub struct JobWorkerConfig {
    /// Maximum number of jobs to claim at once
    pub batch_size: i64,
    /// How long to wait when no jobs are available (max)
    pub max_poll_interval: Duration,
    /// Minimum poll interval
    pub min_poll_interval: Duration,
    /// How often to send heartbeats for running jobs
    pub heartbeat_interval: Duration,
    /// Worker ID for this instance
    pub worker_id: String,
}

impl Default for JobWorkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            max_poll_interval: Duration::from_secs(30),
            min_poll_interval: Duration::from_millis(100),
            heartbeat_interval: Duration::from_secs(30),
            worker_id: format!("worker-{}", Uuid::new_v4()),
        }
    }
}

impl JobWorkerConfig {
    /// Create a new config with a specific worker ID.
    pub fn with_worker_id(worker_id: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            ..Default::default()
        }
    }
}

/// Handler trait for processing claimed jobs using seesaw commands.
///
/// This allows different dispatching strategies (e.g., using seesaw Dispatcher
/// directly vs. legacy handler adapters during migration).
#[async_trait::async_trait]
pub trait SeesawCommandHandler: Send + Sync {
    /// Execute a deserialized command.
    async fn execute(&self, cmd: Box<dyn seesaw::AnyCommand>) -> Result<()>;
}

/// A job worker that processes commands from a queue.
///
/// This worker:
/// 1. Polls the JobStore for ready jobs
/// 2. Deserializes job payloads using CommandRegistry
/// 3. Dispatches commands via SeesawCommandHandler
/// 4. Marks jobs as succeeded/failed
pub struct JobWorker<S: JobStore> {
    store: Arc<S>,
    registry: Arc<CommandRegistry>,
    handler: Arc<dyn SeesawCommandHandler>,
    config: JobWorkerConfig,
    /// Track running jobs for cancellation
    running_jobs: Arc<RwLock<HashMap<Uuid, CancellationToken>>>,
}

impl<S: JobStore + 'static> JobWorker<S> {
    /// Create a new job worker.
    pub fn new(
        store: Arc<S>,
        registry: Arc<CommandRegistry>,
        handler: Arc<dyn SeesawCommandHandler>,
    ) -> Self {
        Self {
            store,
            registry,
            handler,
            config: JobWorkerConfig::default(),
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        store: Arc<S>,
        registry: Arc<CommandRegistry>,
        handler: Arc<dyn SeesawCommandHandler>,
        config: JobWorkerConfig,
    ) -> Self {
        Self {
            store,
            registry,
            handler,
            config,
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process a single claimed job.
    async fn process_job(&self, job: ClaimedJob, shutdown: &CancellationToken) {
        let job_id = job.id;
        let job_type = job.job_type.clone();

        // Create cancellation token for this job
        let job_cancel = shutdown.child_token();

        // Register as running
        {
            let mut running = self.running_jobs.write().await;
            running.insert(job_id, job_cancel.clone());
        }

        // Deserialize command
        let cmd = match self.registry.deserialize(&job) {
            Ok(cmd) => cmd,
            Err(e) => {
                let (error_msg, kind) = match &e {
                    DeserializationError::UnknownCommandType(t) => (
                        format!("unknown command type: {}", t),
                        FailureKind::NonRetryable,
                    ),
                    DeserializationError::UnsupportedVersion { job_type, version } => (
                        format!("unsupported version {} for {}", version, job_type),
                        FailureKind::NonRetryable,
                    ),
                    DeserializationError::InvalidPayload(e) => {
                        (format!("invalid payload: {}", e), FailureKind::NonRetryable)
                    }
                };

                error!(job_id = %job_id, job_type = %job_type, error = %e, "failed to deserialize job");
                if let Err(e) = self.store.mark_failed(job_id, &error_msg, kind).await {
                    error!(job_id = %job_id, error = %e, "failed to mark job as failed");
                }
                self.running_jobs.write().await.remove(&job_id);
                return;
            }
        };

        // Execute with heartbeat
        let result = self
            .execute_with_heartbeat(job_id, cmd, job_cancel.clone())
            .await;

        // Handle result
        match result {
            Ok(()) => {
                debug!(job_id = %job_id, job_type = %job_type, "job succeeded");
                if let Err(e) = self.store.mark_succeeded(job_id).await {
                    error!(job_id = %job_id, error = %e, "failed to mark job as succeeded");
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                let kind = if shutdown.is_cancelled() {
                    // Graceful shutdown - allow retry
                    FailureKind::Retryable
                } else {
                    // Normal failure - allow retry
                    FailureKind::Retryable
                };
                warn!(job_id = %job_id, job_type = %job_type, error = %e, "job failed");
                if let Err(e) = self.store.mark_failed(job_id, &error_msg, kind).await {
                    error!(job_id = %job_id, error = %e, "failed to mark job as failed");
                }
            }
        }

        // Cleanup
        self.running_jobs.write().await.remove(&job_id);
    }

    /// Execute a command with periodic heartbeats.
    async fn execute_with_heartbeat(
        &self,
        job_id: Uuid,
        cmd: Box<dyn seesaw::AnyCommand>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let store = self.store.clone();
        let heartbeat_interval = self.config.heartbeat_interval;

        // Spawn heartbeat task
        let heartbeat_cancel = cancel.clone();
        let heartbeat_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(heartbeat_interval);
            interval.tick().await; // Skip first immediate tick

            loop {
                tokio::select! {
                    _ = heartbeat_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if let Err(e) = store.heartbeat(job_id).await {
                            warn!(job_id = %job_id, error = %e, "heartbeat failed");
                        }
                    }
                }
            }
        });

        // Execute the command
        let result = self.handler.execute(cmd).await;

        // Stop heartbeat
        cancel.cancel();
        let _ = heartbeat_handle.await;

        result
    }
}

#[async_trait::async_trait]
impl<S: JobStore + 'static> Service for JobWorker<S> {
    fn name(&self) -> &'static str {
        "job-worker"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        info!(
            worker_id = %self.config.worker_id,
            batch_size = self.config.batch_size,
            "job worker starting"
        );

        loop {
            // Check for shutdown
            if shutdown.is_cancelled() {
                break;
            }

            // Claim jobs
            let jobs = match self
                .store
                .claim_ready(&self.config.worker_id, self.config.batch_size)
                .await
            {
                Ok(jobs) => jobs,
                Err(e) => {
                    error!(error = %e, "failed to claim jobs");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            if jobs.is_empty() {
                // No jobs available, sleep
                let interval = self.config.max_poll_interval;

                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = tokio::time::sleep(interval) => {}
                }
                continue;
            }

            debug!(count = jobs.len(), "claimed jobs");

            // Process jobs concurrently
            let mut handles = Vec::with_capacity(jobs.len());
            for job in jobs {
                let worker = &self;
                let shutdown_ref = &shutdown;

                handles.push(async move {
                    worker.process_job(job, shutdown_ref).await;
                });
            }

            // Wait for all jobs to complete
            futures::future::join_all(handles).await;
        }

        // Wait for any running jobs to complete
        let running_count = self.running_jobs.read().await.len();
        if running_count > 0 {
            info!(
                count = running_count,
                "waiting for running jobs to complete"
            );

            // Cancel all running jobs
            {
                let running = self.running_jobs.read().await;
                for token in running.values() {
                    token.cancel();
                }
            }

            // Wait for them to finish (with timeout)
            let timeout = Duration::from_secs(30);
            let start = std::time::Instant::now();
            while !self.running_jobs.read().await.is_empty() && start.elapsed() < timeout {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        info!(worker_id = %self.config.worker_id, "job worker stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = JobWorkerConfig::default();
        assert_eq!(config.batch_size, 10);
        assert!(config.worker_id.starts_with("worker-"));
    }

    #[test]
    fn test_config_with_worker_id() {
        let config = JobWorkerConfig::with_worker_id("my-worker");
        assert_eq!(config.worker_id, "my-worker");
    }
}
