//! Job execution helpers for crawling jobs.
//!
//! This module provides a simple execution pattern that:
//! 1. Creates a Job record for tracking
//! 2. Runs the crawl/regenerate logic immediately (synchronously)
//! 3. Updates the Job status on completion
//!
//! Jobs run immediately when called - no polling delay.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};
use uuid::Uuid;

use crate::common::{AppState, WebsiteId};
use crate::domains::crawling::actions::ingest_website;
use crate::kernel::jobs::{Job, JobStatus};
use crate::kernel::ServerDeps;

use super::{CrawlWebsiteJob, RegeneratePostsJob};

/// Result of executing a job
#[derive(Debug, Clone)]
pub struct JobExecutionResult {
    pub job_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Execute a CrawlWebsiteJob with job tracking.
///
/// Creates a Job record, runs the crawl immediately, and updates the status.
/// Returns the job execution result with final status.
pub async fn execute_crawl_website_job(
    job: CrawlWebsiteJob,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<JobExecutionResult> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    // Create a job record for tracking
    let db_job = Job::builder()
        .reference_id(job.website_id)
        .job_type(CrawlWebsiteJob::JOB_TYPE)
        .args(serde_json::to_value(&job)?)
        .status(JobStatus::Running)
        .build();

    let db_job = db_job.insert_with_pool(&ctx.deps().db_pool).await?;
    let job_id = db_job.id;

    info!(
        job_id = %job_id,
        website_id = %website_id,
        "Starting crawl website job"
    );

    // Run the actual crawl
    let result = ingest_website(job.website_id, job.visitor_id, job.use_firecrawl, ctx).await;

    // Update job status based on result and return execution result
    match result {
        Ok(crawl_result) => {
            info!(
                job_id = %job_id,
                status = %crawl_result.status,
                "Crawl website job completed"
            );

            // Mark job as succeeded
            sqlx::query("UPDATE jobs SET status = 'succeeded', updated_at = NOW() WHERE id = $1")
                .bind(job_id)
                .execute(&ctx.deps().db_pool)
                .await?;

            Ok(JobExecutionResult {
                job_id,
                status: crawl_result.status,
                message: crawl_result.message,
            })
        }
        Err(e) => {
            error!(
                job_id = %job_id,
                error = %e,
                "Crawl website job failed"
            );

            // Mark job as failed
            sqlx::query(
                "UPDATE jobs SET status = 'failed', error_message = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(e.to_string())
            .bind(job_id)
            .execute(&ctx.deps().db_pool)
            .await?;

            Ok(JobExecutionResult {
                job_id,
                status: "failed".to_string(),
                message: Some(e.to_string()),
            })
        }
    }
}

/// Execute a RegeneratePostsJob with job tracking.
///
/// Creates a Job record, runs the regeneration immediately, and updates the status.
/// Returns the job execution result with final status.
pub async fn execute_regenerate_posts_job(
    job: RegeneratePostsJob,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<JobExecutionResult> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    // Create a job record for tracking
    let db_job = Job::builder()
        .reference_id(job.website_id)
        .job_type(RegeneratePostsJob::JOB_TYPE)
        .args(serde_json::to_value(&job)?)
        .status(JobStatus::Running)
        .build();

    let db_job = db_job.insert_with_pool(&ctx.deps().db_pool).await?;
    let job_id = db_job.id;

    info!(
        job_id = %job_id,
        website_id = %website_id,
        "Starting regenerate posts job"
    );

    // Import the action
    use crate::domains::crawling::actions::regenerate_posts;

    // Run the actual regeneration immediately
    let is_admin = ctx.next_state().is_admin;
    let result = regenerate_posts(job.website_id, job.visitor_id, is_admin, ctx).await;

    // Update job status based on result and return execution result
    match result {
        Ok(regen_result) => {
            info!(
                job_id = %job_id,
                status = %regen_result.status,
                "Regenerate posts job completed"
            );

            sqlx::query("UPDATE jobs SET status = 'succeeded', updated_at = NOW() WHERE id = $1")
                .bind(job_id)
                .execute(&ctx.deps().db_pool)
                .await?;

            Ok(JobExecutionResult {
                job_id,
                status: regen_result.status,
                message: regen_result.message,
            })
        }
        Err(e) => {
            error!(
                job_id = %job_id,
                error = %e,
                "Regenerate posts job failed"
            );

            sqlx::query(
                "UPDATE jobs SET status = 'failed', error_message = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(e.to_string())
            .bind(job_id)
            .execute(&ctx.deps().db_pool)
            .await?;

            Ok(JobExecutionResult {
                job_id,
                status: "failed".to_string(),
                message: Some(e.to_string()),
            })
        }
    }
}

/// Query result for job status
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub job_id: Uuid,
    pub job_type: String,
    pub status: String,
    pub error_message: Option<String>,
}

impl JobInfo {
    /// Find the latest job for a website by job type
    pub async fn find_latest_for_website(
        website_id: Uuid,
        job_type: &str,
        pool: &sqlx::PgPool,
    ) -> Result<Option<Self>> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
            r#"
            SELECT id, job_type, status::text, error_message
            FROM jobs
            WHERE reference_id = $1 AND job_type = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(website_id)
        .bind(job_type)
        .fetch_optional(pool)
        .await?;

        Ok(
            row.map(|(job_id, job_type, status, error_message)| JobInfo {
                job_id,
                job_type,
                status,
                error_message,
            }),
        )
    }
}
