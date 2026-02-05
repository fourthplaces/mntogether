//! Cascade event handlers for crawling domain
//!
//! These handlers are THIN job enqueuers - they don't run business logic inline.
//! Instead, they enqueue the next job in the pipeline.
//!
//! ## Pipeline Flow
//!
//! ```text
//! WebsiteIngested → handle_enqueue_extract_posts → ExtractPostsJob
//! PostsExtractedFromPages → handle_enqueue_sync_posts → SyncPostsJob
//! PostsSynced → (terminal, no handler)
//! ```
//!
//! The job executors (in `jobs/executor.rs`) run the actual business logic.

use anyhow::Result;
use tracing::info;

use crate::common::{ExtractedPost, JobId, WebsiteId};
use crate::domains::crawling::events::PageExtractionResult;
use crate::domains::crawling::jobs::{
    execute_extract_posts_job, execute_sync_posts_job, ExtractPostsJob, SyncPostsJob,
};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

// ============================================================================
// Job Enqueue Handlers (Thin - just enqueue the next job)
// ============================================================================

/// Enqueue an ExtractPostsJob when website ingestion completes.
///
/// This is a THIN handler - it just creates and executes the extraction job.
/// The actual extraction logic is in `execute_extract_posts_job`.
/// Returns the extracted posts and page results for the next stage.
pub async fn handle_enqueue_extract_posts(
    website_id: WebsiteId,
    job_id: JobId,
    deps: &ServerDeps,
) -> Result<(Vec<ExtractedPost>, Vec<PageExtractionResult>)> {
    info!(
        website_id = %website_id,
        parent_job_id = %job_id,
        "Enqueueing extract posts job"
    );

    let job = ExtractPostsJob::with_parent(website_id.into_uuid(), job_id.into_uuid());
    let result = execute_extract_posts_job(job, deps).await?;

    info!(
        website_id = %website_id,
        extract_job_id = %result.job_id,
        status = %result.status,
        "Extract posts job completed"
    );

    Ok((result.posts, result.page_results))
}

/// Enqueue a SyncPostsJob when post extraction completes.
///
/// This is a THIN handler - it just creates and executes the sync job.
/// The actual sync logic is in `execute_sync_posts_job`.
pub async fn handle_enqueue_sync_posts(
    website_id: WebsiteId,
    job_id: JobId,
    posts: Vec<ExtractedPost>,
    _page_results: Vec<PageExtractionResult>,
    deps: &ServerDeps,
) -> Result<()> {
    info!(
        website_id = %website_id,
        parent_job_id = %job_id,
        posts_count = posts.len(),
        "Enqueueing sync posts job"
    );

    // Use simple sync by default (not LLM sync)
    let job = SyncPostsJob::new(website_id.into_uuid(), posts)
        .with_parent(job_id.into_uuid());
    let result = execute_sync_posts_job(job, deps).await?;

    info!(
        website_id = %website_id,
        sync_job_id = %result.job_id,
        status = %result.status,
        "Sync posts job completed"
    );

    Ok(())
}

// ============================================================================
// Other Handlers
// ============================================================================

/// Mark website as having no posts.
/// Returns the total attempts count for logging.
pub async fn handle_mark_no_posts(
    website_id: WebsiteId,
    _job_id: JobId,
    deps: &ServerDeps,
) -> Result<i32> {
    info!(website_id = %website_id, "Marking website as having no posts");

    let website = Website::find_by_id(website_id, &deps.db_pool).await?;
    let total_attempts = website.crawl_attempt_count.unwrap_or(0);

    Ok(total_attempts)
}
