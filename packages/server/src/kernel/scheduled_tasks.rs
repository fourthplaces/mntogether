//! Scheduled background tasks using tokio-cron-scheduler.
//!
//! This module provides periodic tasks that run on schedules:
//! - Periodic scraping of organization sources
//! - Other scheduled maintenance tasks
//!
//! # Architecture
//!
//! Scheduled tasks run independently of the job queue system.
//! They typically dispatch events or enqueue jobs rather than doing work directly.
//!
//! ```text
//! Scheduler (every hour)
//!     │
//!     └─► find_due_for_scraping()
//!             └─► For each source → dispatch ScrapeSourceRequested event
//!                     └─► Machine → Commands → Effects
//! ```

use anyhow::Result;
use seesaw::EventBus;
use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::domains::member::models::member::Member;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::OrganizationSource;

/// Start all scheduled tasks
pub async fn start_scheduler(pool: PgPool, bus: EventBus) -> Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    // Periodic scraping task - runs every hour
    let scrape_pool = pool.clone();
    let scrape_bus = bus.clone();
    let scrape_job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let pool = scrape_pool.clone();
        let bus = scrape_bus.clone();
        Box::pin(async move {
            if let Err(e) = run_periodic_scrape(&pool, &bus).await {
                tracing::error!("Periodic scrape task failed: {}", e);
            }
        })
    })?;

    scheduler.add(scrape_job).await?;

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
        "Scheduled tasks started (periodic scraping every hour, weekly reset every Monday)"
    );
    Ok(scheduler)
}

/// Run periodic scraping task
///
/// Queries all sources due for scraping and dispatches scrape events.
/// Each scrape runs asynchronously via the event system.
async fn run_periodic_scrape(pool: &PgPool, bus: &EventBus) -> Result<()> {
    tracing::info!("Running periodic scrape task");

    // Find sources due for scraping
    let sources = OrganizationSource::find_due_for_scraping(pool).await?;

    if sources.is_empty() {
        tracing::info!("No sources due for scraping");
        return Ok(());
    }

    tracing::info!("Found {} sources due for scraping", sources.len());

    // Emit scrape event for each source
    for source in sources {
        let job_id = Uuid::new_v4();

        // Emit event (fire-and-forget, non-blocking)
        // System-initiated scrapes use system user ID (all zeros) with admin privileges
        bus.emit(OrganizationEvent::ScrapeSourceRequested {
            source_id: source.id,
            job_id,
            requested_by: Uuid::nil(), // System user
            is_admin: true,            // System has admin privileges
        });

        tracing::info!(
            "Queued scrape job {} for source {} ({})",
            job_id,
            source.id,
            source.organization_name
        );
    }

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
