use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::common::{MemberId, NeedId, SourceId};

/// Organization need - an opportunity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationNeed {
    pub id: NeedId,
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

    // Vector search (for semantic matching)
    pub embedding: Option<pgvector::Vector>,

    // Location coordinates (inherited from organization for proximity matching)
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Submission tracking
    pub submission_type: Option<String>, // 'scraped' | 'user_submitted'
    pub submitted_by_member_id: Option<MemberId>,
    pub submitted_from_ip: Option<String>, // INET stored as string

    // Sync tracking (for scraped needs)
    pub source_id: Option<SourceId>,
    pub source_url: Option<String>, // Specific page URL where need was found
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
    pub async fn find_by_id(id: NeedId, pool: &PgPool) -> Result<Self> {
        let need =
            sqlx::query_as::<_, OrganizationNeed>("SELECT * FROM organization_needs WHERE id = $1")
                .bind(id)
                .fetch_one(pool)
                .await?;
        Ok(need)
    }

    /// Find needs by status
    pub async fn find_by_status(
        status: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let needs = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs
             WHERE status = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(needs)
    }

    /// Find needs by source ID
    pub async fn find_by_source_id(source_id: SourceId, pool: &PgPool) -> Result<Vec<Self>> {
        let needs = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE source_id = $1",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?;
        Ok(needs)
    }

    /// Find need by content hash
    pub async fn find_by_content_hash(content_hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE content_hash = $1 LIMIT 1",
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
                submission_type, submitted_by_member_id, submitted_from_ip,
                source_id, last_seen_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
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
        .bind(self.submitted_by_member_id)
        .bind(self.submitted_from_ip.clone())
        .bind(self.source_id)
        .bind(self.last_seen_at)
        .bind(self.created_at)
        .bind(self.updated_at)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Update need status
    pub async fn update_status(id: NeedId, status: &str, pool: &PgPool) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            UPDATE organization_needs
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(status)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Update need content (for edit + approve)
    pub async fn update_content(
        id: NeedId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            UPDATE organization_needs
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                description_markdown = COALESCE($4, description_markdown),
                tldr = COALESCE($5, tldr),
                contact_info = COALESCE($6, contact_info),
                urgency = COALESCE($7, urgency),
                location = COALESCE($8, location),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(description_markdown)
        .bind(tldr)
        .bind(contact_info)
        .bind(urgency)
        .bind(location)
        .fetch_one(pool)
        .await?;
        Ok(need)
    }

    /// Mark needs as disappeared (for sync)
    pub async fn mark_disappeared(need_ids: &[NeedId], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE organization_needs
            SET disappeared_at = NOW(), status = 'expired', updated_at = NOW()
            WHERE id = ANY($1) AND disappeared_at IS NULL
            "#,
        )
        .bind(need_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update last_seen_at timestamp
    pub async fn update_last_seen(need_ids: &[NeedId], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE organization_needs
            SET last_seen_at = NOW(), updated_at = NOW()
            WHERE id = ANY($1)
            "#,
        )
        .bind(need_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update need embedding
    pub async fn update_embedding(id: NeedId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE organization_needs SET embedding = $2 WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Create a new need (returns inserted record with defaults applied)
    ///
    /// This method handles need creation for both scraped and user-submitted needs.
    /// Last_seen_at is set to NOW() automatically by the database default.
    pub async fn create(
        organization_name: String,
        title: String,
        description: String,
        tldr: String,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        status: String,
        content_hash: String,
        submission_type: Option<String>,
        submitted_by_member_id: Option<MemberId>,
        submitted_from_ip: Option<String>,
        source_id: Option<SourceId>,
        source_url: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            INSERT INTO organization_needs (
                organization_name,
                title,
                description,
                tldr,
                contact_info,
                urgency,
                location,
                status,
                content_hash,
                submission_type,
                submitted_by_member_id,
                submitted_from_ip,
                source_id,
                source_url
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::inet, $13, $14)
            RETURNING *
            "#,
        )
        .bind(organization_name)
        .bind(title)
        .bind(description)
        .bind(tldr)
        .bind(contact_info)
        .bind(urgency)
        .bind(location)
        .bind(status)
        .bind(content_hash)
        .bind(submission_type)
        .bind(submitted_by_member_id)
        .bind(submitted_from_ip)
        .bind(source_id)
        .bind(source_url)
        .fetch_one(pool)
        .await?;

        Ok(need)
    }

    /// Find existing active needs from a source (for sync)
    pub async fn find_active_by_source(source_id: SourceId, pool: &PgPool) -> Result<Vec<Self>> {
        let needs = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            SELECT *
            FROM organization_needs
            WHERE source_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
            "#,
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?;
        Ok(needs)
    }

    /// Find need by source and title (for sync - detecting changed needs)
    pub async fn find_by_source_and_title(
        source_id: SourceId,
        title: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            r#"
            SELECT *
            FROM organization_needs
            WHERE source_id = $1
              AND title = $2
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(source_id)
        .bind(title)
        .fetch_optional(pool)
        .await?;
        Ok(need)
    }

    /// Mark needs as disappeared that are not in the provided content hash list (for sync)
    pub async fn mark_disappeared_except(
        source_id: SourceId,
        content_hashes: &[String],
        pool: &PgPool,
    ) -> Result<Vec<NeedId>> {
        let disappeared_ids = sqlx::query_scalar::<_, NeedId>(
            r#"
            UPDATE organization_needs
            SET disappeared_at = NOW(), updated_at = NOW()
            WHERE source_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
              AND content_hash NOT IN (SELECT * FROM UNNEST($2::text[]))
            RETURNING id
            "#,
        )
        .bind(source_id)
        .bind(content_hashes)
        .fetch_all(pool)
        .await?;
        Ok(disappeared_ids)
    }

    /// Update last_seen_at for a specific need
    pub async fn touch_last_seen(id: NeedId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE organization_needs
            SET last_seen_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Count needs by status (for pagination)
    pub async fn count_by_status(status: &str, pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM organization_needs
            WHERE status = $1
            "#,
        )
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Find need by content hash with status filter (for duplicate detection)
    ///
    /// Returns the NeedId of an existing need with the same content hash
    /// that is either pending_approval or active (not rejected/expired).
    pub async fn find_id_by_content_hash_active(
        content_hash: &str,
        pool: &PgPool,
    ) -> Result<Option<NeedId>> {
        let id = sqlx::query_scalar::<_, NeedId>(
            r#"
            SELECT id
            FROM organization_needs
            WHERE content_hash = $1
              AND status IN ('pending_approval', 'active')
            LIMIT 1
            "#,
        )
        .bind(content_hash)
        .fetch_optional(pool)
        .await?;
        Ok(id)
    }

    // =============================================================================
    // Validation Methods - Business logic validation
    // =============================================================================

    /// Ensure need is active (for operations that require active status)
    ///
    /// Returns an error if the need is not active, preventing operations
    /// like post creation on inactive needs.
    pub fn ensure_active(&self) -> Result<()> {
        if self.status != "active" {
            anyhow::bail!(
                "Need must be active to perform this operation (current status: {})",
                self.status
            );
        }
        Ok(())
    }

    /// Delete a need by ID
    pub async fn delete(id: NeedId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organization_needs WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
