//! Job handlers for crawling domain.
//!
//! These handlers are registered with the JobRegistry and called by the JobRunner.
//! Each handler calls the appropriate action and enqueues follow-up jobs if needed.
//!
//! ## Pipeline Flow
//!
//! ```text
//! CrawlWebsiteJob    → ingest_website()        → enqueue ExtractPostsJob
//! ExtractPostsJob    → extract_posts_for_domain() → enqueue SyncPostsJob
//! SyncPostsJob       → sync_and_deduplicate_posts() → terminal
//! RegeneratePostsJob → regenerate_posts()      → enqueue ExtractPostsJob
//! ```

use std::sync::Arc;

use anyhow::{anyhow, Result};
use tracing::info;

use crate::common::WebsiteId;
use crate::domains::crawling::actions::{ingest_website, regenerate_posts, regenerate_single_post, sync_and_deduplicate_posts};
use crate::domains::crawling::actions::post_extraction::extract_posts_for_domain;
use crate::domains::crawling::jobs::{CrawlWebsiteJob, ExtractPostsJob, RegeneratePostsJob, RegenerateSinglePostJob, SyncPostsJob};
use crate::domains::posts::actions::llm_sync::llm_sync_posts;
use crate::domains::website::models::Website;
use crate::kernel::jobs::JobQueueExt;
use crate::kernel::ServerDeps;

/// Handle CrawlWebsiteJob.
///
/// Ingests website pages and enqueues extraction job on success.
pub async fn handle_crawl_website(job: CrawlWebsiteJob, deps: Arc<ServerDeps>) -> Result<()> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    info!(
        website_id = %website_id,
        use_firecrawl = job.use_firecrawl,
        "Handling crawl website job"
    );

    // Run the crawl - is_admin=true for background jobs
    let _event = ingest_website(
        job.website_id,
        job.visitor_id,
        job.use_firecrawl,
        true, // is_admin
        &deps,
    )
    .await?;

    info!(website_id = %website_id, "Crawl completed, enqueueing extraction");

    // Enqueue extraction job
    deps.jobs
        .enqueue(ExtractPostsJob::new(job.website_id))
        .await?;

    Ok(())
}

/// Handle ExtractPostsJob.
///
/// Extracts posts from ingested pages and enqueues sync job with the results.
pub async fn handle_extract_posts(job: ExtractPostsJob, deps: Arc<ServerDeps>) -> Result<()> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    info!(
        website_id = %website_id,
        parent_job_id = ?job.parent_job_id,
        "Handling extract posts job"
    );

    // Fetch website for domain
    let website = Website::find_by_id(website_id, &deps.db_pool).await?;

    // Get extraction service
    let extraction = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow!("Extraction service not available"))?;

    // Run extraction
    let result = extract_posts_for_domain(&website.domain, extraction.as_ref(), &deps).await?;

    let posts_count = result.posts.len();
    info!(
        website_id = %website_id,
        posts_count = posts_count,
        "Extraction completed, enqueueing sync"
    );

    if !result.posts.is_empty() {
        // Enqueue sync job with extracted posts
        deps.jobs
            .enqueue(SyncPostsJob::new(job.website_id, result.posts))
            .await?;
    } else {
        info!(website_id = %website_id, "No posts found, skipping sync");
    }

    Ok(())
}

/// Handle SyncPostsJob.
///
/// Syncs extracted posts to the database. Terminal job - no follow-up.
pub async fn handle_sync_posts(job: SyncPostsJob, deps: Arc<ServerDeps>) -> Result<()> {
    let website_id = WebsiteId::from_uuid(job.website_id);
    let posts_count = job.extracted_posts.len();

    info!(
        website_id = %website_id,
        posts_count = posts_count,
        use_llm_sync = job.use_llm_sync,
        parent_job_id = ?job.parent_job_id,
        "Handling sync posts job"
    );

    if job.use_llm_sync {
        // LLM-based intelligent sync
        let result = llm_sync_posts(
            website_id,
            job.extracted_posts,
            deps.ai.as_ref(),
            &deps.db_pool,
        )
        .await?;

        info!(
            website_id = %website_id,
            inserted = result.inserted,
            updated = result.updated,
            deleted = result.deleted,
            merged = result.merged,
            "LLM sync completed"
        );
    } else {
        // Simple delete-and-replace sync
        let result = sync_and_deduplicate_posts(website_id, job.extracted_posts, &deps).await?;

        info!(
            website_id = %website_id,
            inserted = result.sync_result.inserted,
            updated = result.sync_result.updated,
            deleted = result.sync_result.deleted,
            merged = result.sync_result.merged,
            "Simple sync completed"
        );
    }

    Ok(())
}

/// Handle RegeneratePostsJob.
///
/// Regenerates posts from existing pages and enqueues extraction job.
pub async fn handle_regenerate_posts(job: RegeneratePostsJob, deps: Arc<ServerDeps>) -> Result<()> {
    let website_id = WebsiteId::from_uuid(job.website_id);

    info!(
        website_id = %website_id,
        "Handling regenerate posts job"
    );

    // Run regeneration - is_admin=true for background jobs
    let _event = regenerate_posts(job.website_id, job.visitor_id, true, &deps).await?;

    info!(website_id = %website_id, "Regeneration completed, enqueueing extraction");

    // Enqueue extraction job
    deps.jobs
        .enqueue(ExtractPostsJob::new(job.website_id))
        .await?;

    Ok(())
}

/// Handle RegenerateSinglePostJob.
///
/// Regenerates a single post from its source extraction pages. Terminal job.
pub async fn handle_regenerate_single_post(job: RegenerateSinglePostJob, deps: Arc<ServerDeps>) -> Result<()> {
    info!(
        post_id = %job.post_id,
        "Handling regenerate single post job"
    );

    regenerate_single_post(job.post_id, &deps).await?;

    info!(post_id = %job.post_id, "Single post regeneration complete");

    Ok(())
}

/// Register all crawling job handlers with the registry.
///
/// Call this at startup to register handlers for all crawling jobs.
pub fn register_crawling_jobs(registry: &mut crate::kernel::jobs::JobRegistry) {
    registry.register::<CrawlWebsiteJob, _, _>(CrawlWebsiteJob::JOB_TYPE, |job, deps| async move {
        handle_crawl_website(job, deps).await
    });

    registry.register::<ExtractPostsJob, _, _>(ExtractPostsJob::JOB_TYPE, |job, deps| async move {
        handle_extract_posts(job, deps).await
    });

    registry.register::<SyncPostsJob, _, _>(SyncPostsJob::JOB_TYPE, |job, deps| async move {
        handle_sync_posts(job, deps).await
    });

    registry.register::<RegeneratePostsJob, _, _>(RegeneratePostsJob::JOB_TYPE, |job, deps| async move {
        handle_regenerate_posts(job, deps).await
    });

    registry.register::<RegenerateSinglePostJob, _, _>(RegenerateSinglePostJob::JOB_TYPE, |job, deps| async move {
        handle_regenerate_single_post(job, deps).await
    });
}
