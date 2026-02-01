use crate::common::entity_ids::{Id, PostId, MemberId};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

pub struct PostReport;
pub type PostReportId = Id<PostReport>;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostReportRecord {
    pub id: PostReportId,
    pub post_id: PostId,
    pub reported_by: Option<MemberId>,
    pub reporter_email: Option<String>,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub resolved_by: Option<MemberId>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub action_taken: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostReportWithDetails {
    pub id: PostReportId,
    pub post_id: PostId,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub action_taken: Option<String>,
    pub post_title: String,
    pub organization_name: String,
    pub post_type: String,
    pub post_status: String,
    pub report_count_for_post: i64,
}

impl PostReportRecord {
    pub async fn create(
        post_id: PostId,
        reported_by: Option<MemberId>,
        reporter_email: Option<String>,
        reason: String,
        category: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let reported_by_uuid = reported_by.map(|id| id.into_uuid());

        sqlx::query_as::<_, Self>(
            "INSERT INTO post_reports (post_id, reported_by, reporter_email, reason, category)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(post_id.into_uuid())
        .bind(reported_by_uuid)
        .bind(reporter_email)
        .bind(reason)
        .bind(category)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn query_pending(
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<PostReportWithDetails>> {
        sqlx::query_as::<_, PostReportWithDetails>(
            "SELECT id, post_id, reason, category, status, created_at,
                    resolved_at, resolution_notes, action_taken,
                    post_title, organization_name, post_type, post_status,
                    report_count_for_post
             FROM post_reports_with_details
             WHERE status = 'pending'
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn query_all(
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<PostReportWithDetails>> {
        sqlx::query_as::<_, PostReportWithDetails>(
            "SELECT id, post_id, reason, category, status, created_at,
                    resolved_at, resolution_notes, action_taken,
                    post_title, organization_name, post_type, post_status,
                    report_count_for_post
             FROM post_reports_with_details
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn query_for_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_reports
             WHERE post_id = $1
             ORDER BY created_at DESC",
        )
        .bind(post_id.into_uuid())
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn resolve(
        id: PostReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        action_taken: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE post_reports
             SET status = 'resolved', resolved_by = $2, resolved_at = NOW(),
                 resolution_notes = $3, action_taken = $4, updated_at = NOW()
             WHERE id = $1
             RETURNING *",
        )
        .bind(id.into_uuid())
        .bind(resolved_by.into_uuid())
        .bind(resolution_notes)
        .bind(action_taken)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn dismiss(
        id: PostReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE post_reports
             SET status = 'dismissed', resolved_by = $2, resolved_at = NOW(),
                 resolution_notes = $3, updated_at = NOW()
             WHERE id = $1
             RETURNING *",
        )
        .bind(id.into_uuid())
        .bind(resolved_by.into_uuid())
        .bind(resolution_notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
