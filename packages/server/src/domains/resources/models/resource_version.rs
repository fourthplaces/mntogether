//! ResourceVersion model - audit trail for resource changes
//!
//! Every change to a resource creates a new version record. This provides
//! complete history of how content has evolved and why.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::common::{ResourceId, ResourceVersionId};

/// Reason for creating a version
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeReason {
    Created,    // Initial creation
    AiUpdate,   // AI detected content change and updated
    ManualEdit, // Admin manually edited
    AiMerge,    // AI merged content from multiple sources
}

impl std::fmt::Display for ChangeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeReason::Created => write!(f, "created"),
            ChangeReason::AiUpdate => write!(f, "ai_update"),
            ChangeReason::ManualEdit => write!(f, "manual_edit"),
            ChangeReason::AiMerge => write!(f, "ai_merge"),
        }
    }
}

impl std::str::FromStr for ChangeReason {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "created" => Ok(ChangeReason::Created),
            "ai_update" => Ok(ChangeReason::AiUpdate),
            "manual_edit" => Ok(ChangeReason::ManualEdit),
            "ai_merge" => Ok(ChangeReason::AiMerge),
            _ => Err(anyhow::anyhow!("Invalid change reason: {}", s)),
        }
    }
}

/// Deduplication decision context (stored when AI makes update/merge decisions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupDecision {
    pub matched_resource_id: Option<uuid::Uuid>,
    pub similarity_score: Option<f32>,
    pub ai_reasoning: Option<String>,
}

/// ResourceVersion - snapshot of a resource at a point in time
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResourceVersion {
    pub id: ResourceVersionId,
    pub resource_id: ResourceId,

    // Snapshot of content
    pub title: String,
    pub content: String,
    pub location: Option<String>,

    // Why this version was created
    pub change_reason: String,

    // Deduplication context (for ai_update/ai_merge)
    pub dedup_decision: Option<JsonValue>,

    pub created_at: DateTime<Utc>,
}

impl ResourceVersion {
    /// Find all versions for a resource (most recent first)
    pub async fn find_by_resource_id(resource_id: ResourceId, pool: &PgPool) -> Result<Vec<Self>> {
        let versions = sqlx::query_as::<_, Self>(
            "SELECT * FROM resource_versions WHERE resource_id = $1 ORDER BY created_at DESC",
        )
        .bind(resource_id)
        .fetch_all(pool)
        .await?;
        Ok(versions)
    }

    /// Find the latest version for a resource
    pub async fn find_latest(resource_id: ResourceId, pool: &PgPool) -> Result<Option<Self>> {
        let version = sqlx::query_as::<_, Self>(
            "SELECT * FROM resource_versions WHERE resource_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(resource_id)
        .fetch_optional(pool)
        .await?;
        Ok(version)
    }

    /// Find version by ID
    pub async fn find_by_id(id: ResourceVersionId, pool: &PgPool) -> Result<Option<Self>> {
        let version = sqlx::query_as::<_, Self>("SELECT * FROM resource_versions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(version)
    }

    /// Create a new version record
    pub async fn create(
        resource_id: ResourceId,
        title: String,
        content: String,
        location: Option<String>,
        change_reason: ChangeReason,
        dedup_decision: Option<DedupDecision>,
        pool: &PgPool,
    ) -> Result<Self> {
        let dedup_json = dedup_decision
            .map(|d| serde_json::to_value(d).ok())
            .flatten();

        let version = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO resource_versions (resource_id, title, content, location, change_reason, dedup_decision)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(resource_id)
        .bind(title)
        .bind(content)
        .bind(location)
        .bind(change_reason.to_string())
        .bind(dedup_json)
        .fetch_one(pool)
        .await?;
        Ok(version)
    }

    /// Get dedup decision as typed struct
    pub fn get_dedup_decision(&self) -> Option<DedupDecision> {
        self.dedup_decision
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Count versions for a resource
    pub async fn count_by_resource_id(resource_id: ResourceId, pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resource_versions WHERE resource_id = $1",
        )
        .bind(resource_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}
