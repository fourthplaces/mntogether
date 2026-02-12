use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{MemberId, OrganizationId};

/// Organization status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationStatus {
    PendingReview,
    Approved,
    Rejected,
    Suspended,
}

impl std::fmt::Display for OrganizationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationStatus::PendingReview => write!(f, "pending_review"),
            OrganizationStatus::Approved => write!(f, "approved"),
            OrganizationStatus::Rejected => write!(f, "rejected"),
            OrganizationStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for OrganizationStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_review" => Ok(OrganizationStatus::PendingReview),
            "approved" => Ok(OrganizationStatus::Approved),
            "rejected" => Ok(OrganizationStatus::Rejected),
            "suspended" => Ok(OrganizationStatus::Suspended),
            _ => Err(anyhow::anyhow!("Invalid organization status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,

    // Approval workflow
    pub status: String,
    pub submitted_by: Option<MemberId>,
    pub submitter_type: Option<String>,
    pub submission_context: Option<String>,
    pub reviewed_by: Option<MemberId>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    pub last_extracted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    pub async fn create(
        name: &str,
        description: Option<&str>,
        submitter_type: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO organizations (name, description, submitter_type, status) VALUES ($1, $2, $3, 'pending_review') RETURNING *",
        )
        .bind(name)
        .bind(description)
        .bind(submitter_type)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM organizations ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find all approved organizations that have at least one active post
    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT DISTINCT o.*
            FROM organizations o
            JOIN sources s ON s.organization_id = o.id
            JOIN post_sources ps ON ps.source_id = s.id
            JOIN posts p ON p.id = ps.post_id
            WHERE o.status = 'approved'
              AND p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
            ORDER BY o.name ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find organizations pending review
    pub async fn find_pending(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM organizations WHERE status = 'pending_review' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(
        id: OrganizationId,
        name: &str,
        description: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE organizations SET name = $2, description = $3, updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Approve an organization
    pub async fn approve(id: OrganizationId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE organizations
            SET
                status = 'approved',
                reviewed_by = $2,
                reviewed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Reject an organization
    pub async fn reject(
        id: OrganizationId,
        reviewed_by: MemberId,
        rejection_reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE organizations
            SET
                status = 'rejected',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(rejection_reason)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Suspend an approved organization
    pub async fn suspend(
        id: OrganizationId,
        reviewed_by: MemberId,
        reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE organizations
            SET
                status = 'suspended',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by)
        .bind(reason)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Move an organization back to pending review
    pub async fn move_to_pending(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE organizations
            SET
                status = 'pending_review',
                reviewed_by = NULL,
                reviewed_at = NULL,
                rejection_reason = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find or create an organization by name (exact match).
    /// If exists, updates description only if the new one is non-null.
    /// System-created orgs default to 'pending_review'.
    pub async fn find_or_create_by_name(
        name: &str,
        description: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO organizations (name, description, submitter_type, status)
            VALUES ($1, $2, 'system', 'pending_review')
            ON CONFLICT (name) DO UPDATE
            SET description = COALESCE(EXCLUDED.description, organizations.description)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find orgs where any source was crawled more recently than the last extraction.
    pub async fn find_needing_extraction(pool: &PgPool) -> Result<Vec<OrganizationId>> {
        sqlx::query_scalar::<_, OrganizationId>(
            r#"
            SELECT DISTINCT o.id
            FROM organizations o
            JOIN sources s ON s.organization_id = o.id
            WHERE s.status = 'approved' AND s.active = true
              AND s.last_scraped_at IS NOT NULL
              AND (o.last_extracted_at IS NULL OR s.last_scraped_at > o.last_extracted_at)
            ORDER BY o.id
            LIMIT 10
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_last_extracted(id: OrganizationId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "UPDATE organizations SET last_extracted_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(id: OrganizationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
