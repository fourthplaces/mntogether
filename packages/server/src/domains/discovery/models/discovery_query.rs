//! DiscoveryQuery model
//!
//! Search queries for Tavily-based website discovery, manageable via admin UI.
//! Supports `{location}` placeholder for geographic targeting.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// A search query used for discovering community resource websites via Tavily
#[derive(Debug, Clone, FromRow)]
pub struct DiscoveryQuery {
    pub id: Uuid,
    pub query_text: String,
    pub category: Option<String>,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DiscoveryQuery {
    /// Find all active queries (used by the discovery pipeline)
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_queries WHERE is_active = true ORDER BY category, created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all queries (including inactive, for admin UI)
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_queries ORDER BY category, created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a query by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM discovery_queries WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Create a new discovery query
    pub async fn create(
        query_text: String,
        category: Option<String>,
        created_by: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO discovery_queries (query_text, category, created_by)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(query_text)
        .bind(category)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update query text and/or category
    pub async fn update(
        id: Uuid,
        query_text: String,
        category: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE discovery_queries
             SET query_text = $2, category = $3, updated_at = NOW()
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(query_text)
        .bind(category)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Toggle active/inactive
    pub async fn toggle_active(id: Uuid, is_active: bool, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE discovery_queries
             SET is_active = $2, updated_at = NOW()
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(is_active)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete a query (results in discovery_run_results are preserved via CASCADE)
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM discovery_queries WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
