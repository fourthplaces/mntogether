//! Job execution helpers for crawling jobs.
//!
//! This module provides a simple execution pattern that:
//! 1. Creates a Job record for tracking
//! 2. Runs the job logic immediately (synchronously)
//! 3. Updates the Job status on completion
//! 4. Emits events to trigger the next job in the pipeline
//!
//! Jobs run immediately when called - no polling delay.
//!
//! ## Pipeline Flow
//!
//! ```text
//! CrawlWebsiteJob → WebsiteIngested event → ExtractPostsJob
//! ExtractPostsJob → PostsExtractedFromPages event → SyncPostsJob
//! SyncPostsJob → PostsSynced event (terminal)
//! ```

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};
use uuid::Uuid;

use crate::common::{AppState, JobId, WebsiteId};
use crate::domains::crawling::actions::ingest_website;
use crate::domains::crawling::actions::post_extraction::extract_posts_for_domain;
use crate::domains::crawling::actions::sync_and_deduplicate_posts;
use crate::domains::crawling::events::{CrawlEvent, PageExtractionResult};
use crate::domains::posts::actions::llm_sync::llm_sync_posts;
use crate::domains::website::models::Website;
use crate::kernel::jobs::{Job, JobStatus};
use crate::kernel::ServerDeps;

use super::{CrawlWebsiteJob, ExtractPostsJob, RegeneratePostsJob, SyncPostsJob};

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

/// Execute an ExtractPostsJob with job tracking.
///
/// Creates a Job record, runs the three-pass extraction, and updates the status.
/// Emits `PostsExtractedFromPages` event on success to trigger sync job.
pub async fn execute_extract_posts_job(
    job: ExtractPostsJob,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<JobExecutionResult> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    // Create a job record for tracking
    let db_job = Job::builder()
        .reference_id(job.website_id)
        .job_type(ExtractPostsJob::JOB_TYPE)
        .args(serde_json::to_value(&job)?)
        .status(JobStatus::Running)
        .timeout_ms(600_000) // 10 minutes for extraction
        .build();

    let db_job = db_job.insert_with_pool(&ctx.deps().db_pool).await?;
    let job_id = db_job.id;
    let event_job_id = JobId::from_uuid(job_id);

    info!(
        job_id = %job_id,
        website_id = %website_id,
        parent_job_id = ?job.parent_job_id,
        "Starting extract posts job"
    );

    // Fetch website
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            error!(job_id = %job_id, error = %e, "Website not found");
            mark_job_failed(job_id, &e.to_string(), &ctx.deps().db_pool).await?;
            return Ok(JobExecutionResult {
                job_id,
                status: "failed".to_string(),
                message: Some(e.to_string()),
            });
        }
    };

    // Get extraction service
    let extraction = match ctx.deps().extraction.as_ref() {
        Some(e) => e,
        None => {
            let err = "Extraction service not available";
            error!(job_id = %job_id, err);
            mark_job_failed(job_id, err, &ctx.deps().db_pool).await?;
            return Ok(JobExecutionResult {
                job_id,
                status: "failed".to_string(),
                message: Some(err.to_string()),
            });
        }
    };

    // Run three-pass extraction
    let result = extract_posts_for_domain(&website.domain, extraction.as_ref(), ctx.deps()).await;

    match result {
        Ok(extraction_result) => {
            let posts_count = extraction_result.posts.len();
            info!(
                job_id = %job_id,
                posts_count = posts_count,
                "Extract posts job completed"
            );

            // Mark job as succeeded
            sqlx::query("UPDATE jobs SET status = 'succeeded', updated_at = NOW() WHERE id = $1")
                .bind(job_id)
                .execute(&ctx.deps().db_pool)
                .await?;

            // Build page results for the event
            let page_results: Vec<PageExtractionResult> = extraction_result
                .page_urls
                .iter()
                .map(|url| PageExtractionResult {
                    url: url.clone(),
                    snapshot_id: None,
                    listings_count: 0,
                    has_posts: true,
                })
                .collect();

            // Emit event to trigger sync job
            ctx.emit(CrawlEvent::PostsExtractedFromPages {
                website_id,
                job_id: event_job_id,
                posts: extraction_result.posts,
                page_results,
            });

            Ok(JobExecutionResult {
                job_id,
                status: "succeeded".to_string(),
                message: Some(format!("Extracted {} posts", posts_count)),
            })
        }
        Err(e) => {
            error!(job_id = %job_id, error = %e, "Extract posts job failed");
            mark_job_failed(job_id, &e.to_string(), &ctx.deps().db_pool).await?;

            Ok(JobExecutionResult {
                job_id,
                status: "failed".to_string(),
                message: Some(e.to_string()),
            })
        }
    }
}

/// Execute a SyncPostsJob with job tracking.
///
/// Creates a Job record, runs sync (simple or LLM), and updates the status.
/// Emits `PostsSynced` event on success (terminal event).
pub async fn execute_sync_posts_job(
    job: SyncPostsJob,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<JobExecutionResult> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    // Create a job record for tracking
    let db_job = Job::builder()
        .reference_id(job.website_id)
        .job_type(SyncPostsJob::JOB_TYPE)
        .args(serde_json::to_value(&job)?)
        .status(JobStatus::Running)
        .timeout_ms(300_000) // 5 minutes for sync
        .build();

    let db_job = db_job.insert_with_pool(&ctx.deps().db_pool).await?;
    let job_id = db_job.id;
    let event_job_id = JobId::from_uuid(job_id);

    info!(
        job_id = %job_id,
        website_id = %website_id,
        posts_count = job.extracted_posts.len(),
        use_llm_sync = job.use_llm_sync,
        parent_job_id = ?job.parent_job_id,
        "Starting sync posts job"
    );

    // Run sync
    let sync_result = if job.use_llm_sync {
        // LLM-based intelligent sync
        match llm_sync_posts(website_id, job.extracted_posts, ctx.deps().ai.as_ref(), &ctx.deps().db_pool).await {
            Ok(result) => SyncResult {
                inserted: result.inserted,
                updated: result.updated,
                deleted: result.deleted,
                merged: result.merged,
            },
            Err(e) => {
                error!(job_id = %job_id, error = %e, "LLM sync failed");
                mark_job_failed(job_id, &e.to_string(), &ctx.deps().db_pool).await?;
                return Ok(JobExecutionResult {
                    job_id,
                    status: "failed".to_string(),
                    message: Some(e.to_string()),
                });
            }
        }
    } else {
        // Simple delete-and-replace sync
        match sync_and_deduplicate_posts(website_id, job.extracted_posts, ctx.deps()).await {
            Ok(result) => SyncResult {
                inserted: result.sync_result.inserted,
                updated: result.sync_result.updated,
                deleted: result.sync_result.deleted,
                merged: result.sync_result.merged,
            },
            Err(e) => {
                error!(job_id = %job_id, error = %e, "Simple sync failed");
                mark_job_failed(job_id, &e.to_string(), &ctx.deps().db_pool).await?;
                return Ok(JobExecutionResult {
                    job_id,
                    status: "failed".to_string(),
                    message: Some(e.to_string()),
                });
            }
        }
    };

    info!(
        job_id = %job_id,
        inserted = sync_result.inserted,
        updated = sync_result.updated,
        deleted = sync_result.deleted,
        merged = sync_result.merged,
        "Sync posts job completed"
    );

    // Mark job as succeeded
    sqlx::query("UPDATE jobs SET status = 'succeeded', updated_at = NOW() WHERE id = $1")
        .bind(job_id)
        .execute(&ctx.deps().db_pool)
        .await?;

    // Emit terminal event
    ctx.emit(CrawlEvent::PostsSynced {
        website_id,
        job_id: event_job_id,
        new_count: sync_result.inserted,
        updated_count: sync_result.updated,
        unchanged_count: sync_result.deleted + sync_result.merged,
    });

    Ok(JobExecutionResult {
        job_id,
        status: "succeeded".to_string(),
        message: Some(format!(
            "Synced {} new, {} updated, {} deleted, {} merged",
            sync_result.inserted, sync_result.updated, sync_result.deleted, sync_result.merged
        )),
    })
}

/// Internal sync result for job reporting
struct SyncResult {
    inserted: usize,
    updated: usize,
    deleted: usize,
    merged: usize,
}

/// Helper to mark a job as failed
async fn mark_job_failed(job_id: Uuid, error: &str, pool: &sqlx::PgPool) -> Result<()> {
    sqlx::query(
        "UPDATE jobs SET status = 'failed', error_message = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(error)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
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
