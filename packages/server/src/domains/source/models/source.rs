use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{MemberId, OrganizationId, PaginationDirection, SourceId, ValidatedPaginationArgs};

/// Source - a unified content source (website, social profile, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Source {
    pub id: SourceId,
    pub source_type: String, // 'website', 'instagram', 'facebook', 'tiktok'
    pub url: Option<String>,
    pub organization_id: Option<OrganizationId>,
    pub status: String, // 'pending_review', 'approved', 'rejected', 'suspended'
    pub active: bool,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub submitted_by: Option<Uuid>,
    pub submitter_type: Option<String>,
    pub submission_context: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Source status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    PendingReview,
    Approved,
    Rejected,
    Suspended,
}

impl std::fmt::Display for SourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceStatus::PendingReview => write!(f, "pending_review"),
            SourceStatus::Approved => write!(f, "approved"),
            SourceStatus::Rejected => write!(f, "rejected"),
            SourceStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for SourceStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_review" => Ok(SourceStatus::PendingReview),
            "approved" => Ok(SourceStatus::Approved),
            "rejected" => Ok(SourceStatus::Rejected),
            "suspended" => Ok(SourceStatus::Suspended),
            _ => Err(anyhow::anyhow!("Invalid source status: {}", s)),
        }
    }
}

impl Source {
    pub async fn find_by_id(id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM sources WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_id_optional(id: SourceId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM sources WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sources WHERE active = true AND status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sources WHERE status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_pending_review(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sources WHERE status = 'pending_review' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sources
            WHERE status = 'approved'
              AND active = true
              AND (last_scraped_at IS NULL
                   OR last_scraped_at < NOW() - (scrape_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_scraped_at NULLS FIRST
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_organization(org_id: OrganizationId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sources WHERE organization_id = $1 ORDER BY source_type, created_at",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_without_organization(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sources WHERE organization_id IS NULL AND status = 'approved' ORDER BY created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn approve(id: SourceId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sources
            SET status = 'approved', reviewed_by = $2, reviewed_at = NOW(), updated_at = NOW()
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

    pub async fn reject(
        id: SourceId,
        reviewed_by: MemberId,
        rejection_reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sources
            SET status = 'rejected', reviewed_by = $2, reviewed_at = NOW(),
                rejection_reason = $3, updated_at = NOW()
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

    pub async fn suspend(
        id: SourceId,
        reviewed_by: MemberId,
        reason: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE sources
            SET status = 'suspended', reviewed_by = $2, reviewed_at = NOW(),
                rejection_reason = $3, updated_at = NOW()
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

    pub async fn set_organization_id(
        id: SourceId,
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE sources SET organization_id = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn unset_organization_id(id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE sources SET organization_id = NULL, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_last_scraped(id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE sources SET last_scraped_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn set_active(id: SourceId, active: bool, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE sources SET active = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(active)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: SourceId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM sources WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find sources with cursor-based pagination
    pub async fn find_paginated(
        status: Option<&str>,
        source_type: Option<&str>,
        search: Option<&str>,
        organization_id: Option<Uuid>,
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    r#"
                    SELECT s.* FROM sources s
                    LEFT JOIN website_sources ws ON ws.source_id = s.id
                    LEFT JOIN social_sources ss ON ss.source_id = s.id
                    WHERE ($1::text IS NULL OR s.status = $1)
                      AND ($2::uuid IS NULL OR s.id > $2)
                      AND ($4::text IS NULL OR ws.domain ILIKE '%' || $4 || '%' OR ss.handle ILIKE '%' || $4 || '%')
                      AND ($5::uuid IS NULL OR s.organization_id = $5)
                      AND ($6::text IS NULL OR s.source_type = $6)
                    ORDER BY s.id ASC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(search)
                .bind(organization_id)
                .bind(source_type)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                let mut rows = sqlx::query_as::<_, Self>(
                    r#"
                    SELECT s.* FROM sources s
                    LEFT JOIN website_sources ws ON ws.source_id = s.id
                    LEFT JOIN social_sources ss ON ss.source_id = s.id
                    WHERE ($1::text IS NULL OR s.status = $1)
                      AND ($2::uuid IS NULL OR s.id < $2)
                      AND ($4::text IS NULL OR ws.domain ILIKE '%' || $4 || '%' OR ss.handle ILIKE '%' || $4 || '%')
                      AND ($5::uuid IS NULL OR s.organization_id = $5)
                      AND ($6::text IS NULL OR s.source_type = $6)
                    ORDER BY s.id DESC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(search)
                .bind(organization_id)
                .bind(source_type)
                .fetch_all(pool)
                .await?;

                rows.reverse();
                rows
            }
        };

        let has_more = results.len() > args.limit as usize;
        let results = if has_more {
            results.into_iter().take(args.limit as usize).collect()
        } else {
            results
        };

        Ok((results, has_more))
    }

    pub async fn count_with_filters(
        status: Option<&str>,
        source_type: Option<&str>,
        search: Option<&str>,
        organization_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM sources s
            LEFT JOIN website_sources ws ON ws.source_id = s.id
            LEFT JOIN social_sources ss ON ss.source_id = s.id
            WHERE ($1::text IS NULL OR s.status = $1)
              AND ($2::text IS NULL OR ws.domain ILIKE '%' || $2 || '%' OR ss.handle ILIKE '%' || $2 || '%')
              AND ($3::uuid IS NULL OR s.organization_id = $3)
              AND ($4::text IS NULL OR s.source_type = $4)
            "#,
        )
        .bind(status)
        .bind(search)
        .bind(organization_id)
        .bind(source_type)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
