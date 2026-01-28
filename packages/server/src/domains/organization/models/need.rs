use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

/// Organization need - a volunteer opportunity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationNeed {
    pub id: Uuid,
    pub organization_name: String,

    // Content
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,

    // Contact
    pub contact_info: Option<JsonValue>,

    // Metadata
    pub urgency: Option<String>,
    pub status: String, // Maps to NeedStatus enum in edges
    pub content_hash: Option<String>,
    pub location: Option<String>,

    // Submission tracking
    pub submission_type: Option<String>, // 'scraped' | 'user_submitted'
    pub submitted_by_volunteer_id: Option<Uuid>,
    pub submitted_from_ip: Option<std::net::IpAddr>,

    // Sync tracking (for scraped needs)
    pub source_id: Option<Uuid>,
    pub last_seen_at: DateTime<Utc>,
    pub disappeared_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status enum for type-safe edges
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NeedStatus {
    PendingApproval,
    Active,
    Rejected,
    Expired,
}

impl std::fmt::Display for NeedStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NeedStatus::PendingApproval => write!(f, "pending_approval"),
            NeedStatus::Active => write!(f, "active"),
            NeedStatus::Rejected => write!(f, "rejected"),
            NeedStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for NeedStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_approval" => Ok(NeedStatus::PendingApproval),
            "active" => Ok(NeedStatus::Active),
            "rejected" => Ok(NeedStatus::Rejected),
            "expired" => Ok(NeedStatus::Expired),
            _ => Err(anyhow::anyhow!("Invalid need status: {}", s)),
        }
    }
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl OrganizationNeed {
    /// Find need by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Find needs by status
    pub async fn find_by_status(status: &str, limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<Self>> {
        let needs = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs
             WHERE status = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3"
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(needs)
    }

    /// Find needs by source ID
    pub async fn find_by_source_id(source_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let needs = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE source_id = $1"
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?;
        Ok(needs)
    }

    /// Find need by content hash
    pub async fn find_by_content_hash(content_hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE content_hash = $1 LIMIT 1"
        )
        .bind(content_hash)
        .fetch_optional(pool)
        .await?;
        Ok(need)
    }

    /// Insert new need
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            INSERT INTO organization_needs (
                id, organization_name, title, description, description_markdown, tldr,
                contact_info, urgency, status, content_hash, location,
                submission_type, submitted_by_volunteer_id, submitted_from_ip,
                source_id, last_seen_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#
        )
        .bind(self.id)
        .bind(&self.organization_name)
        .bind(&self.title)
        .bind(&self.description)
        .bind(&self.description_markdown)
        .bind(&self.tldr)
        .bind(&self.contact_info)
        .bind(&self.urgency)
        .bind(&self.status)
        .bind(&self.content_hash)
        .bind(&self.location)
        .bind(&self.submission_type)
        .bind(self.submitted_by_volunteer_id)
        .bind(self.submitted_from_ip)
        .bind(self.source_id)
        .bind(self.last_seen_at)
        .bind(self.created_at)
        .bind(self.updated_at)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Update need status
    pub async fn update_status(id: Uuid, status: &str, pool: &PgPool) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            UPDATE organization_needs
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#
        )
        .bind(status)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Update need content (for edit + approve)
    pub async fn update_content(
        id: Uuid,
        title: Option<String>,
        description: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            UPDATE organization_needs
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                contact_info = COALESCE($4, contact_info),
                urgency = COALESCE($5, urgency),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(contact_info)
        .bind(urgency)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Mark needs as disappeared (for sync)
    pub async fn mark_disappeared(need_ids: &[Uuid], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE organization_needs
            SET disappeared_at = NOW(), status = 'expired', updated_at = NOW()
            WHERE id = ANY($1) AND disappeared_at IS NULL
            "#
        )
        .bind(need_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update last_seen_at timestamp
    pub async fn update_last_seen(need_ids: &[Uuid], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE organization_needs
            SET last_seen_at = NOW(), updated_at = NOW()
            WHERE id = ANY($1)
            "#
        )
        .bind(need_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
