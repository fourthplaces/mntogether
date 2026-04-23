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
    pub source_type: String,

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

    // True when this row was inserted by the dev seed script (data/seed.mjs).
    // Surfaced to the admin CMS so every dummy entity is visibly labeled.
    pub is_seed: bool,
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

    pub async fn create_with_source_type(
        name: &str,
        description: Option<&str>,
        submitter_type: &str,
        source_type: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO organizations (name, description, submitter_type, source_type, status) VALUES ($1, $2, $3, $4, 'pending_review') RETURNING *",
        )
        .bind(name)
        .bind(description)
        .bind(submitter_type)
        .bind(source_type)
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

    pub async fn list_by_source_type(source_type: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM organizations WHERE source_type = $1 ORDER BY name ASC",
        )
        .bind(source_type)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all approved organizations that have at least one active post.
    ///
    /// Public-facing (exposed via `/Organizations/public_list` and the
    /// web-app org directory). Filters out seed organizations AND seed
    /// posts, so dummy content can't leak onto the public site even if
    /// the seeder has run against the live DB.
    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT DISTINCT o.*
            FROM organizations o
            JOIN sources s ON s.organization_id = o.id
            JOIN post_sources ps ON ps.source_id = s.id
            JOIN posts p ON p.id = ps.post_id
            WHERE o.status = 'approved'
              AND o.is_seed = false
              AND p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND p.is_seed = false
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

    pub async fn delete(id: OrganizationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // Ingest-time dedup lookups (spec §7.1).
    // ========================================================================

    /// Find an organization whose website source's domain matches the supplied
    /// domain (case-insensitive, already-normalised by the caller — no `www.`
    /// prefix, no trailing slash). Traverses
    /// `organizations → sources → website_sources`. Returns the most-recently-
    /// created match when multiple orgs share a domain (rare but possible
    /// before dedup lands).
    pub async fn find_by_website_domain(
        domain: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT o.*
            FROM organizations o
            JOIN sources s ON s.organization_id = o.id
            JOIN website_sources ws ON ws.source_id = s.id
            WHERE LOWER(ws.domain) = LOWER($1)
            ORDER BY o.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(domain)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Exact name match (case-insensitive). Spec §7.1 step 3.
    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM organizations
            WHERE LOWER(name) = LOWER($1)
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// The primary source for this organization (the one post_sources rows
    /// should point at). Prefers website sources, falls back to any source.
    pub async fn primary_source_id(
        id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Option<uuid::Uuid>> {
        sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            SELECT s.id FROM sources s
            WHERE s.organization_id = $1
            ORDER BY
                CASE s.source_type WHEN 'website' THEN 0 ELSE 1 END,
                s.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Ensure a website source exists for this organization. Called during
    /// ingest when a submission carries a website but no matching source row
    /// is yet linked. Returns the source_id ready to feed into post_sources.
    pub async fn ensure_website_source(
        id: OrganizationId,
        domain: &str,
        url: &str,
        pool: &PgPool,
    ) -> Result<uuid::Uuid> {
        // First: is there already a source for this domain?
        if let Some(existing) = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            SELECT s.id FROM sources s
            JOIN website_sources ws ON ws.source_id = s.id
            WHERE s.organization_id = $1 AND LOWER(ws.domain) = LOWER($2)
            LIMIT 1
            "#,
        )
        .bind(id)
        .bind(domain)
        .fetch_optional(pool)
        .await?
        {
            return Ok(existing);
        }

        // Insert parent `sources` row.
        let source_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO sources (source_type, url, organization_id, status, active)
            VALUES ('website', $1, $2, 'approved', true)
            RETURNING id
            "#,
        )
        .bind(url)
        .bind(id)
        .fetch_one(pool)
        .await?;

        // Insert child `website_sources` row. If the domain is already used by
        // another org (rare), fall through — the post_sources row will still
        // reference a valid parent source.
        let _ = sqlx::query(
            r#"
            INSERT INTO website_sources (source_id, domain)
            VALUES ($1, $2)
            ON CONFLICT (domain) DO NOTHING
            "#,
        )
        .bind(source_id)
        .bind(domain)
        .execute(pool)
        .await?;

        Ok(source_id)
    }

    /// Enrich an existing org with new submission metadata. Only fills
    /// columns that are currently NULL (matches §7.1 "enrich NULL fields"
    /// semantics). `name` is updated only when the stored row's name is
    /// empty, which shouldn't normally happen but is cheap to guard.
    pub async fn enrich_if_null(
        id: OrganizationId,
        name: &str,
        description: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE organizations
            SET
                name = CASE WHEN name IS NULL OR name = '' THEN $2 ELSE name END,
                description = COALESCE(description, $3),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
