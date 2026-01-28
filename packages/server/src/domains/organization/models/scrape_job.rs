use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Scrape job status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scrape_job_status", rename_all = "lowercase")]
pub enum ScrapeJobStatus {
    Pending,
    Scraping,
    Extracting,
    Syncing,
    Completed,
    Failed,
}

impl ToString for ScrapeJobStatus {
    fn to_string(&self) -> String {
        match self {
            ScrapeJobStatus::Pending => "pending".to_string(),
            ScrapeJobStatus::Scraping => "scraping".to_string(),
            ScrapeJobStatus::Extracting => "extracting".to_string(),
            ScrapeJobStatus::Syncing => "syncing".to_string(),
            ScrapeJobStatus::Completed => "completed".to_string(),
            ScrapeJobStatus::Failed => "failed".to_string(),
        }
    }
}

/// Scrape job - tracks async scraping workflow
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScrapeJob {
    pub id: Uuid,
    pub source_id: Uuid,
    pub status: ScrapeJobStatus,
    pub error_message: Option<String>,
    pub scraped_at: Option<DateTime<Utc>>,
    pub extracted_at: Option<DateTime<Utc>>,
    pub synced_at: Option<DateTime<Utc>>,
    pub new_needs_count: Option<i32>,
    pub changed_needs_count: Option<i32>,
    pub disappeared_needs_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl ScrapeJob {
    /// Create a new pending scrape job
    pub async fn create(source_id: Uuid, pool: &PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            INSERT INTO scrape_jobs (source_id, status)
            VALUES ($1, 'pending')
            RETURNING *
            "#,
        )
        .bind(source_id)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    /// Find job by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        let job = sqlx::query_as::<_, ScrapeJob>("SELECT * FROM scrape_jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(job)
    }

    /// Update job status
    pub async fn update_status(
        id: Uuid,
        status: ScrapeJobStatus,
        pool: &PgPool,
    ) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            UPDATE scrape_jobs
            SET status = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    /// Mark job as scraped
    pub async fn mark_scraped(id: Uuid, pool: &PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            UPDATE scrape_jobs
            SET status = 'scraping', scraped_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    /// Mark job as extracted
    pub async fn mark_extracted(id: Uuid, pool: &PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            UPDATE scrape_jobs
            SET status = 'extracting', extracted_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    /// Mark job as syncing
    pub async fn mark_syncing(id: Uuid, pool: &PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            UPDATE scrape_jobs
            SET status = 'syncing', synced_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    /// Mark job as completed with results
    pub async fn mark_completed(
        id: Uuid,
        new_count: i32,
        changed_count: i32,
        disappeared_count: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            UPDATE scrape_jobs
            SET status = 'completed',
                completed_at = NOW(),
                new_needs_count = $2,
                changed_needs_count = $3,
                disappeared_needs_count = $4
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(new_count)
        .bind(changed_count)
        .bind(disappeared_count)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    /// Mark job as failed with error
    pub async fn mark_failed(id: Uuid, error: &str, pool: &PgPool) -> Result<Self> {
        let job = sqlx::query_as::<_, ScrapeJob>(
            r#"
            UPDATE scrape_jobs
            SET status = 'failed',
                completed_at = NOW(),
                error_message = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(error)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }
}
