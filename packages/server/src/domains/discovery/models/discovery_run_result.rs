//! DiscoveryRunResult model
//!
//! Individual results from discovery runs — provides full lineage
//! from query → discovered URL → filter result → website (if created).

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// A single discovered website from a discovery run
#[derive(Debug, Clone, FromRow)]
pub struct DiscoveryRunResult {
    pub id: Uuid,
    pub run_id: Uuid,
    pub query_id: Uuid,
    pub domain: String,
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub relevance_score: Option<f64>,
    pub filter_result: String,
    pub filter_reason: Option<String>,
    pub website_id: Option<Uuid>,
    pub discovered_at: DateTime<Utc>,
}

impl DiscoveryRunResult {
    /// Create a batch of results for a run
    pub async fn create(
        run_id: Uuid,
        query_id: Uuid,
        domain: String,
        url: String,
        title: Option<String>,
        snippet: Option<String>,
        relevance_score: Option<f64>,
        filter_result: &str,
        filter_reason: Option<String>,
        website_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO discovery_run_results
             (run_id, query_id, domain, url, title, snippet, relevance_score,
              filter_result, filter_reason, website_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        )
        .bind(run_id)
        .bind(query_id)
        .bind(domain)
        .bind(url)
        .bind(title)
        .bind(snippet)
        .bind(relevance_score)
        .bind(filter_result)
        .bind(filter_reason)
        .bind(website_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all results for a discovery run
    pub async fn find_by_run(run_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_run_results
             WHERE run_id = $1
             ORDER BY query_id, relevance_score DESC NULLS LAST",
        )
        .bind(run_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all results for a specific query (across all runs)
    pub async fn find_by_query(query_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_run_results
             WHERE query_id = $1
             ORDER BY discovered_at DESC",
        )
        .bind(query_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find discovery sources for a website (reverse lineage)
    pub async fn find_by_website(website_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_run_results
             WHERE website_id = $1
             ORDER BY discovered_at DESC",
        )
        .bind(website_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
