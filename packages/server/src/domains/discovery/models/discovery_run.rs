//! DiscoveryRun model
//!
//! Tracks each execution of the discovery pipeline (scheduled or manual).

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// A single execution of the discovery pipeline
#[derive(Debug, Clone, FromRow)]
pub struct DiscoveryRun {
    pub id: Uuid,
    pub queries_executed: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_filtered: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub trigger_type: String,
}

impl DiscoveryRun {
    /// Create a new discovery run record
    pub async fn create(trigger_type: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO discovery_runs (trigger_type) VALUES ($1) RETURNING *",
        )
        .bind(trigger_type)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Mark run as completed with final stats
    pub async fn complete(
        id: Uuid,
        queries_executed: i32,
        total_results: i32,
        websites_created: i32,
        websites_filtered: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE discovery_runs
             SET queries_executed = $2, total_results = $3,
                 websites_created = $4, websites_filtered = $5,
                 completed_at = NOW()
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(queries_executed)
        .bind(total_results)
        .bind(websites_created)
        .bind(websites_filtered)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a run by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM discovery_runs WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Find recent runs (for admin UI)
    pub async fn find_recent(limit: i32, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_runs ORDER BY started_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
