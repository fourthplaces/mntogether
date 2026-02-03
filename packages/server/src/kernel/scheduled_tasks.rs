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

use crate::common::{AppState, MemberId};
use crate::config::Config;
use crate::domains::member::models::member::Member;
use crate::domains::posts::actions as post_actions;
use crate::domains::posts::effects::run_discovery_searches;
use crate::domains::website::models::Website;
use crate::kernel::TavilyClient;
use crate::server::graphql::context::AppEngine;

/// Start all scheduled tasks
///
/// In seesaw 0.6.0, we use engine.activate() to emit events instead of EventBus.
pub async fn start_scheduler(pool: PgPool, engine: Arc<AppEngine>) -> Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    // Periodic scraping task - runs every hour
    let scrape_pool = pool.clone();
    let scrape_engine = engine.clone();
    let scrape_job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let pool = scrape_pool.clone();
        let engine = scrape_engine.clone();
        Box::pin(async move {
            if let Err(e) = run_periodic_scrape(&pool, &engine).await {
                tracing::error!("Periodic scrape task failed: {}", e);
            }
        })
    })?;

    scheduler.add(scrape_job).await?;

    // Periodic search task - runs every hour
    let search_pool = pool.clone();
    let search_job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let pool = search_pool.clone();
        Box::pin(async move {
            if let Err(e) = run_periodic_searches(&pool).await {
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
/// Queries all sources due for scraping and dispatches scrape events.
/// Each scrape runs asynchronously via the event system.
async fn run_periodic_scrape(pool: &PgPool, engine: &Arc<AppEngine>) -> Result<()> {
    tracing::info!("Running periodic scrape task");

    // Find sources due for scraping
    let sources = Website::find_due_for_scraping(pool).await?;

    if sources.is_empty() {
        tracing::info!("No sources due for scraping");
        return Ok(());
    }

    tracing::info!("Found {} sources due for scraping", sources.len());

    // Call scrape action for each source
    for source in sources {
        // Call the action directly via process() - action creates its own job_id
        let result = engine
            .activate(AppState::default())
            .process(|ectx| {
                post_actions::scrape_source(
                    source.id.into_uuid(),
                    MemberId::nil().into_uuid(), // System user
                    true,                        // System has admin privileges
                    ectx,
                )
            })
            .await;

        match result {
            Ok(job_result) => {
                tracing::info!(
                    "Scraped source {} ({}) with job {} - status: {}",
                    source.id,
                    source.domain,
                    job_result.job_id,
                    job_result.status
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to scrape source {} ({}): {}",
                    source.id,
                    source.domain,
                    e
                );
            }
        }
    }

    Ok(())
}

/// Run periodic discovery search task
///
/// Runs static discovery queries via Tavily to find new community resources.
/// Creates pending websites for admin review.
async fn run_periodic_searches(pool: &PgPool) -> Result<()> {
    tracing::info!("Running periodic discovery search task");

    // Load config for Tavily API key
    let config = Config::from_env()?;

    // Create Tavily search client
    let search_service = TavilyClient::new(config.tavily_api_key.clone())?;

    // Run discovery searches with static queries
    let result = run_discovery_searches(&search_service, pool).await?;

    tracing::info!(
        queries_run = result.queries_run,
        total_results = result.total_results,
        websites_created = result.websites_created,
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
