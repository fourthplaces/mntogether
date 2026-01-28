//! Job testing utilities.
//!
//! Re-exports and utilities for testing job execution.

pub use super::manager::{MockJobHandler, ScheduleOptions, TestJobManager};

use anyhow::Result;

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
