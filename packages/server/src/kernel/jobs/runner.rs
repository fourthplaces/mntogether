//! Job runner service for processing background jobs.
//!
//! The `JobRunner` is a background service that:
//! - Polls the database for ready jobs
//! - Deserializes and executes jobs using the registry
//! - Handles status updates (succeeded/failed)
//! - Manages retries via the job queue
//!
//! # Architecture
//!
//! ```text
//! JobRunner
//!     │
//!     ├─► Poll DB (claim jobs via JobQueue)
//!     ├─► Execute via JobRegistry (deserialize + call handler)
//!     └─► Mark succeeded/failed (JobQueue handles retries)
//! ```
//!
//! # Example
//!
//! ```ignore
//! let registry = Arc::new(build_job_registry());
//! let runner = JobRunner::new(job_queue, registry, deps);
//!
//! // Spawn as background task
//! tokio::spawn(runner.run());
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::queue::JobQueue;
use super::registry::SharedJobRegistry;
use super::ErrorKind;
use crate::kernel::ServerDeps;

/// Configuration for the job runner.
#[derive(Debug, Clone)]
pub struct JobRunnerConfig {
    /// Maximum number of jobs to claim at once
    pub batch_size: i64,
    /// How long to wait when no jobs are available
    pub poll_interval: Duration,
    /// Worker ID for this instance
    pub worker_id: String,
}

impl Default for JobRunnerConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            poll_interval: Duration::from_secs(5),
            worker_id: format!("runner-{}", Uuid::new_v4()),
        }
    }
}

impl JobRunnerConfig {
    /// Create a new config with a specific worker ID.
    pub fn with_worker_id(worker_id: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            ..Default::default()
        }
    }
}

/// Background service that processes jobs from the queue.
///
/// The runner polls for jobs, executes them via the registry,
/// and updates their status. Retries are handled automatically
/// by the job queue's `mark_failed` implementation.
pub struct JobRunner {
    job_queue: Arc<dyn JobQueue>,
    registry: SharedJobRegistry,
    deps: Arc<ServerDeps>,
    config: JobRunnerConfig,
    shutdown: Arc<AtomicBool>,
}

impl JobRunner {
    /// Create a new job runner.
    pub fn new(
        job_queue: Arc<dyn JobQueue>,
        registry: SharedJobRegistry,
        deps: Arc<ServerDeps>,
    ) -> Self {
        Self {
            job_queue,
            registry,
            deps,
            config: JobRunnerConfig::default(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        job_queue: Arc<dyn JobQueue>,
        registry: SharedJobRegistry,
        deps: Arc<ServerDeps>,
        config: JobRunnerConfig,
    ) -> Self {
        Self {
            job_queue,
            registry,
            deps,
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a shutdown handle for graceful shutdown.
    ///
    /// Call `store(true, Ordering::SeqCst)` on the returned Arc to signal shutdown.
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        self.shutdown.clone()
    }

    /// Request shutdown of the runner.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been requested.
    fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Run the job runner until shutdown is requested.
    ///
    /// This is the main loop that polls for jobs and executes them.
    /// Call `request_shutdown()` to stop the runner gracefully.
    pub async fn run(self) -> Result<()> {
        info!(
            worker_id = %self.config.worker_id,
            batch_size = self.config.batch_size,
            poll_interval_ms = self.config.poll_interval.as_millis() as u64,
            "job runner starting"
        );

        loop {
            // Check for shutdown
            if self.is_shutdown_requested() {
                break;
            }

            // Claim jobs
            let jobs = match self.job_queue.claim(&self.config.worker_id, self.config.batch_size).await {
                Ok(jobs) => jobs,
                Err(e) => {
                    error!(error = %e, "failed to claim jobs");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            if jobs.is_empty() {
                // No jobs available, sleep until next poll
                tokio::time::sleep(self.config.poll_interval).await;
                continue;
            }

            debug!(count = jobs.len(), "claimed jobs");

            // Process jobs sequentially (can be made concurrent if needed)
            for job in jobs {
                if self.is_shutdown_requested() {
                    break;
                }

                let job_id = job.id;
                let job_type = job.command_type().to_string();

                debug!(job_id = %job_id, job_type = %job_type, "executing job");

                // Execute via registry
                let result = self.registry.execute(&job, self.deps.clone()).await;

                // Update status
                match result {
                    Ok(()) => {
                        info!(job_id = %job_id, job_type = %job_type, "job succeeded");
                        if let Err(e) = self.job_queue.mark_succeeded(job_id).await {
                            error!(job_id = %job_id, error = %e, "failed to mark job as succeeded");
                        }
                    }
                    Err(e) => {
                        warn!(job_id = %job_id, job_type = %job_type, error = %e, "job failed");

                        // Classify error for retry decision
                        let error_kind = classify_error(&e);

                        if let Err(mark_err) = self
                            .job_queue
                            .mark_failed(job_id, &e.to_string(), error_kind)
                            .await
                        {
                            error!(job_id = %job_id, error = %mark_err, "failed to mark job as failed");
                        }
                    }
                }
            }
        }

        info!(worker_id = %self.config.worker_id, "job runner stopped");
        Ok(())
    }

    /// Run until a shutdown signal is received.
    ///
    /// Convenience method that listens for Ctrl+C.
    pub async fn run_until_shutdown(self) -> Result<()> {
        let shutdown = self.shutdown_handle();

        // Spawn signal handler
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            info!("received shutdown signal");
            shutdown.store(true, Ordering::SeqCst);
        });

        self.run().await
    }
}

/// Classify an error to determine retry behavior.
///
/// Returns `Retryable` for transient errors that may succeed on retry,
/// and `NonRetryable` for permanent failures.
fn classify_error(error: &anyhow::Error) -> ErrorKind {
    let error_str = error.to_string().to_lowercase();

    // Non-retryable: validation errors, not found, permission denied
    if error_str.contains("not found")
        || error_str.contains("invalid")
        || error_str.contains("permission denied")
        || error_str.contains("unauthorized")
        || error_str.contains("forbidden")
    {
        return ErrorKind::NonRetryable;
    }

    // Non-retryable: deserialization errors
    if error_str.contains("deserialize") || error_str.contains("parse") {
        return ErrorKind::NonRetryable;
    }

    // Everything else is retryable (network errors, timeouts, etc.)
    ErrorKind::Retryable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = JobRunnerConfig::default();
        assert_eq!(config.batch_size, 10);
        assert!(config.worker_id.starts_with("runner-"));
    }

    #[test]
    fn test_config_with_worker_id() {
        let config = JobRunnerConfig::with_worker_id("my-runner");
        assert_eq!(config.worker_id, "my-runner");
    }

    #[test]
    fn test_classify_error_retryable() {
        let error = anyhow::anyhow!("connection timeout");
        assert_eq!(classify_error(&error), ErrorKind::Retryable);
    }

    #[test]
    fn test_classify_error_not_found() {
        let error = anyhow::anyhow!("website not found");
        assert_eq!(classify_error(&error), ErrorKind::NonRetryable);
    }

    #[test]
    fn test_classify_error_deserialize() {
        let error = anyhow::anyhow!("failed to deserialize payload");
        assert_eq!(classify_error(&error), ErrorKind::NonRetryable);
    }
}
