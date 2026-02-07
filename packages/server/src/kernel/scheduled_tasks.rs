//! Scheduled background tasks using tokio-cron-scheduler.
//!
//! This module provides periodic tasks that run on schedules:
//! - Periodic scraping of organization sources
//! - Other scheduled maintenance tasks
//!
//! # Architecture
//!
//! Scheduled tasks run independently of the job queue system.
//! They typically dispatch events using the engine or do work directly.
//!
//! ```text
//! Scheduler (every hour)
//!     │
//!     └─► find_due_for_scraping()
//!             └─► For each source → emit ScrapeSourceRequested event via engine
//!                     └─► Effects → Business logic
//! ```

use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::common::MemberId;
use crate::domains::crawling::activities::ingest_website;
use crate::domains::discovery::activities::run_discovery;
use crate::domains::member::models::member::Member;
use crate::domains::website::models::Website;

use crate::kernel::ServerDeps;

/// Start all scheduled tasks
///
/// In seesaw 0.7.2, we call actions directly with ServerDeps.
pub async fn start_scheduler(pool: PgPool, server_deps: Arc<ServerDeps>) -> Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    // Periodic scraping task - runs every hour
    let scrape_pool = pool.clone();
    let scrape_deps = server_deps.clone();
    let scrape_job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let pool = scrape_pool.clone();
        let deps = scrape_deps.clone();
        Box::pin(async move {
            if let Err(e) = run_periodic_scrape(&pool, &deps).await {
                tracing::error!("Periodic scrape task failed: {}", e);
            }
        })
    })?;

    scheduler.add(scrape_job).await?;

    // Periodic search task - runs every hour
    let search_deps = server_deps.clone();
    let search_job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let deps = search_deps.clone();
        Box::pin(async move {
            if let Err(e) = run_periodic_searches(&deps).await {
                tracing::error!("Periodic search task failed: {}", e);
            }
        })
    })?;

    scheduler.add(search_job).await?;

    // Weekly notification reset - runs every Monday at midnight
    let reset_pool = pool.clone();
    let reset_job = Job::new_async("0 0 0 * * MON", move |_uuid, _lock| {
        let pool = reset_pool.clone();
        Box::pin(async move {
            if let Err(e) = run_weekly_reset(&pool).await {
                tracing::error!("Weekly reset task failed: {}", e);
            }
        })
    })?;

    scheduler.add(reset_job).await?;
    scheduler.start().await?;

    tracing::info!(
        "Scheduled tasks started (periodic scraping every hour, periodic search every hour, weekly reset every Monday)"
    );
    Ok(scheduler)
}

/// Run periodic scraping task
///
/// Queries all sources due for scraping and calls the ingest action directly.
/// In seesaw 0.7.2, we pass ServerDeps directly to actions.
async fn run_periodic_scrape(pool: &PgPool, deps: &ServerDeps) -> Result<()> {
    tracing::info!("Running periodic scrape task");

    // Find sources due for scraping
    let sources = Website::find_due_for_scraping(pool).await?;

    if sources.is_empty() {
        tracing::info!("No sources due for scraping");
        return Ok(());
    }

    tracing::info!("Found {} sources due for scraping", sources.len());

    // Ingest each website using the extraction library
    for website in sources {
        let result = ingest_website(
            website.id.into_uuid(),
            MemberId::nil().into_uuid(), // System user
            true,                        // Use Firecrawl for better JS rendering
            true,                        // is_admin (system task)
            deps,
        )
        .await;

        match result {
            Ok(result) => {
                tracing::info!(
                    "Ingested website {} ({}) with job {} - pages: {}, summarized: {}",
                    website.id,
                    website.domain,
                    result.job_id,
                    result.pages_crawled,
                    result.pages_summarized
                );
            },
            Err(e) => {
                tracing::error!(
                    "Failed to ingest website {} ({}): {}",
                    website.id,
                    website.domain,
                    e
                );
            }
        }
    }

    Ok(())
}

/// Run periodic discovery search task
///
/// Runs discovery queries from the database via Tavily with AI pre-filtering.
/// Creates pending websites for admin review.
async fn run_periodic_searches(deps: &ServerDeps) -> Result<()> {
    tracing::info!("Running periodic discovery search task");

    let stats = run_discovery("scheduled", deps).await?;

    tracing::info!(
        queries_executed = stats.queries_executed,
        total_results = stats.total_results,
        websites_created = stats.websites_created,
        websites_filtered = stats.websites_filtered,
        "Discovery search completed"
    );

    Ok(())
}

/// Run weekly notification reset task
///
/// Resets notification_count_this_week to 0 for all members every Monday.
/// This ensures members can receive up to 3 notifications per week.
async fn run_weekly_reset(pool: &PgPool) -> Result<()> {
    tracing::info!("Running weekly notification reset task");

    let rows_affected = Member::reset_weekly_counts(pool).await?;

    tracing::info!(
        "Weekly reset complete: reset notification count for {} members",
        rows_affected
    );

    Ok(())
}
