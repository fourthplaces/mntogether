//! DiscoveryFilterRule model
//!
//! Plain-text filter rules evaluated by AI before websites enter the approval queue.
//! Rules with query_id = NULL are global (apply to all queries).
//! Per-query rules override global rules when conflicting.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// A plain-text filter rule for pre-screening discovered websites
#[derive(Debug, Clone, FromRow)]
pub struct DiscoveryFilterRule {
    pub id: Uuid,
    pub query_id: Option<Uuid>,
    pub rule_text: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl DiscoveryFilterRule {
    /// Find all active global rules (query_id IS NULL)
    pub async fn find_global(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_filter_rules
             WHERE query_id IS NULL AND is_active = true
             ORDER BY sort_order",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all rules for a specific query (excludes global rules)
    pub async fn find_by_query(query_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_filter_rules
             WHERE query_id = $1 AND is_active = true
             ORDER BY sort_order",
        )
        .bind(query_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all applicable rules for a query (global + per-query, ordered)
    pub async fn find_applicable(query_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM discovery_filter_rules
             WHERE (query_id IS NULL OR query_id = $1) AND is_active = true
             ORDER BY query_id NULLS FIRST, sort_order",
        )
        .bind(query_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all rules (for admin UI, includes inactive)
    pub async fn find_all_for_query(query_id: Option<Uuid>, pool: &PgPool) -> Result<Vec<Self>> {
        match query_id {
            Some(qid) => sqlx::query_as::<_, Self>(
                "SELECT * FROM discovery_filter_rules
                     WHERE query_id = $1
                     ORDER BY sort_order",
            )
            .bind(qid)
            .fetch_all(pool)
            .await
            .map_err(Into::into),
            None => sqlx::query_as::<_, Self>(
                "SELECT * FROM discovery_filter_rules
                     WHERE query_id IS NULL
                     ORDER BY sort_order",
            )
            .fetch_all(pool)
            .await
            .map_err(Into::into),
        }
    }

    /// Create a new filter rule
    pub async fn create(
        query_id: Option<Uuid>,
        rule_text: String,
        created_by: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        // Auto-increment sort_order based on existing rules
        let max_order: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(sort_order) FROM discovery_filter_rules
             WHERE (query_id IS NOT DISTINCT FROM $1)",
        )
        .bind(query_id)
        .fetch_one(pool)
        .await?;

        let next_order = max_order.0.unwrap_or(0) + 1;

        sqlx::query_as::<_, Self>(
            "INSERT INTO discovery_filter_rules (query_id, rule_text, sort_order, created_by)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(query_id)
        .bind(rule_text)
        .bind(next_order)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update rule text
    pub async fn update(id: Uuid, rule_text: String, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE discovery_filter_rules SET rule_text = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(rule_text)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete a filter rule
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM discovery_filter_rules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
