//! Migration workflow state management
//!
//! Tracks the progress and state of data migrations in the database.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Phase of a migration workflow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowPhase {
    Running,
    Paused,
    Completed,
    Failed,
}

impl WorkflowPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Running,
        }
    }
}

/// Migration workflow database model
#[derive(Debug, Clone, FromRow)]
pub struct MigrationWorkflow {
    pub id: Uuid,
    pub name: String,
    pub phase: String,
    pub total_items: i64,
    pub completed_items: i64,
    pub failed_items: i64,
    pub skipped_items: i64,
    pub last_processed_id: Option<Uuid>,
    pub dry_run: bool,
    pub error_budget: Decimal,
    pub started_at: DateTime<Utc>,
    pub paused_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl MigrationWorkflow {
    /// Get workflow phase as enum
    pub fn phase(&self) -> WorkflowPhase {
        WorkflowPhase::from_str(&self.phase)
    }

    /// Calculate current error rate
    pub fn error_rate(&self) -> f64 {
        let total = self.completed_items + self.failed_items;
        if total == 0 {
            0.0
        } else {
            self.failed_items as f64 / total as f64
        }
    }

    /// Check if error budget is exceeded
    pub fn error_budget_exceeded(&self) -> bool {
        let budget: f64 = self
            .error_budget
            .try_into()
            .unwrap_or(0.01);
        self.error_rate() > budget
    }

    /// Find workflow by name
    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM migration_workflows WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Create a new workflow
    pub async fn create(
        name: &str,
        total_items: i64,
        dry_run: bool,
        error_budget: f64,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO migration_workflows (name, total_items, dry_run, error_budget)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (name) DO UPDATE SET
                phase = 'running',
                total_items = $2,
                completed_items = 0,
                failed_items = 0,
                skipped_items = 0,
                last_processed_id = NULL,
                dry_run = $3,
                error_budget = $4,
                started_at = NOW(),
                paused_at = NULL,
                completed_at = NULL
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(total_items)
        .bind(dry_run)
        .bind(Decimal::try_from(error_budget).unwrap_or_else(|_| Decimal::new(1, 2)))
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update progress after processing a batch
    pub async fn update_progress(
        name: &str,
        completed_delta: i64,
        failed_delta: i64,
        skipped_delta: i64,
        last_processed_id: Uuid,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE migration_workflows
            SET
                completed_items = completed_items + $2,
                failed_items = failed_items + $3,
                skipped_items = skipped_items + $4,
                last_processed_id = $5
            WHERE name = $1
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(completed_delta)
        .bind(failed_delta)
        .bind(skipped_delta)
        .bind(last_processed_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Pause the workflow
    pub async fn pause(name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE migration_workflows
            SET phase = 'paused', paused_at = NOW()
            WHERE name = $1
            RETURNING *
            "#,
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Resume the workflow
    pub async fn resume(name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE migration_workflows
            SET phase = 'running', paused_at = NULL
            WHERE name = $1
            RETURNING *
            "#,
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Mark as completed
    pub async fn complete(name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE migration_workflows
            SET phase = 'completed', completed_at = NOW()
            WHERE name = $1
            RETURNING *
            "#,
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Mark as failed
    pub async fn fail(name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE migration_workflows
            SET phase = 'failed', completed_at = NOW()
            WHERE name = $1
            RETURNING *
            "#,
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
