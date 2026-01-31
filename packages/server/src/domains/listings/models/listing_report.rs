use crate::common::entity_ids::{Id, ListingId, MemberId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use anyhow::Result;

pub struct ListingReport;
pub type ListingReportId = Id<ListingReport>;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ListingReportRecord {
    pub id: ListingReportId,
    pub listing_id: ListingId,
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
pub struct ListingReportWithDetails {
    pub id: ListingReportId,
    pub listing_id: ListingId,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub action_taken: Option<String>,
    pub listing_title: String,
    pub organization_name: String,
    pub listing_type: String,
    pub listing_status: String,
    pub report_count_for_listing: i64,
}

impl ListingReportRecord {
    pub async fn create(
        listing_id: ListingId,
        reported_by: Option<MemberId>,
        reporter_email: Option<String>,
        reason: String,
        category: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let reported_by_uuid = reported_by.map(|id| id.into_uuid());

        sqlx::query_as::<_, Self>(
            "INSERT INTO listing_reports (listing_id, reported_by, reporter_email, reason, category)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(listing_id.into_uuid())
        .bind(reported_by_uuid)
        .bind(reporter_email)
        .bind(reason)
        .bind(category)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn query_pending(limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<ListingReportWithDetails>> {
        sqlx::query_as::<_, ListingReportWithDetails>(
            "SELECT id, listing_id, reason, category, status, created_at,
                    resolved_at, resolution_notes, action_taken,
                    listing_title, organization_name, listing_type, listing_status,
                    report_count_for_listing
             FROM listing_reports_with_details
             WHERE status = 'pending'
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn query_all(limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<ListingReportWithDetails>> {
        sqlx::query_as::<_, ListingReportWithDetails>(
            "SELECT id, listing_id, reason, category, status, created_at,
                    resolved_at, resolution_notes, action_taken,
                    listing_title, organization_name, listing_type, listing_status,
                    report_count_for_listing
             FROM listing_reports_with_details
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn query_for_listing(listing_id: ListingId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM listing_reports
             WHERE listing_id = $1
             ORDER BY created_at DESC"
        )
        .bind(listing_id.into_uuid())
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn resolve(
        id: ListingReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        action_taken: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE listing_reports
             SET status = 'resolved', resolved_by = $2, resolved_at = NOW(),
                 resolution_notes = $3, action_taken = $4, updated_at = NOW()
             WHERE id = $1
             RETURNING *"
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
        id: ListingReportId,
        resolved_by: MemberId,
        resolution_notes: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE listing_reports
             SET status = 'dismissed', resolved_by = $2, resolved_at = NOW(),
                 resolution_notes = $3, updated_at = NOW()
             WHERE id = $1
             RETURNING *"
        )
        .bind(id.into_uuid())
        .bind(resolved_by.into_uuid())
        .bind(resolution_notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
