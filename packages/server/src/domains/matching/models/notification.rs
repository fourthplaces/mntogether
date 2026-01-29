use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ListingId, MemberId};

/// Notification record - tracks when a member was notified about a listing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub member_id: Uuid,
    pub why_relevant: String,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    /// Record a notification (upsert - ignores duplicates)
    ///
    /// Uses ON CONFLICT DO NOTHING to prevent duplicate notifications
    /// for the same listing-member pair.
    pub async fn record(
        listing_id: ListingId,
        member_id: MemberId,
        why_relevant: String,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO notifications (listing_id, member_id, why_relevant)
             VALUES ($1, $2, $3)
             ON CONFLICT (listing_id, member_id) DO NOTHING",
        )
        .bind(listing_id.into_uuid())
        .bind(member_id.into_uuid())
        .bind(why_relevant)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Find all notifications for a member
    pub async fn find_by_member(member_id: MemberId, pool: &PgPool) -> Result<Vec<Self>> {
        let notifications = sqlx::query_as::<_, Notification>(
            "SELECT * FROM notifications WHERE member_id = $1 ORDER BY created_at DESC",
        )
        .bind(member_id.into_uuid())
        .fetch_all(pool)
        .await?;

        Ok(notifications)
    }

    /// Find all notifications for a listing
    pub async fn find_by_listing(listing_id: ListingId, pool: &PgPool) -> Result<Vec<Self>> {
        let notifications = sqlx::query_as::<_, Notification>(
            "SELECT * FROM notifications WHERE listing_id = $1 ORDER BY created_at DESC",
        )
        .bind(listing_id.into_uuid())
        .fetch_all(pool)
        .await?;

        Ok(notifications)
    }
}
