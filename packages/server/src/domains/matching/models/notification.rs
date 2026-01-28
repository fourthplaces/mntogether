use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Notification record - tracks when a member was notified about a need
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub need_id: Uuid,
    pub member_id: Uuid,
    pub why_relevant: String,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    /// Record a notification (upsert - ignores duplicates)
    ///
    /// Uses ON CONFLICT DO NOTHING to prevent duplicate notifications
    /// for the same need-member pair.
    pub async fn record(
        need_id: Uuid,
        member_id: Uuid,
        why_relevant: String,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO notifications (need_id, member_id, why_relevant)
             VALUES ($1, $2, $3)
             ON CONFLICT (need_id, member_id) DO NOTHING",
        )
        .bind(need_id)
        .bind(member_id)
        .bind(why_relevant)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Find all notifications for a member
    pub async fn find_by_member(member_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let notifications = sqlx::query_as::<_, Notification>(
            "SELECT * FROM notifications WHERE member_id = $1 ORDER BY created_at DESC",
        )
        .bind(member_id)
        .fetch_all(pool)
        .await?;

        Ok(notifications)
    }

    /// Find all notifications for a need
    pub async fn find_by_need(need_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let notifications = sqlx::query_as::<_, Notification>(
            "SELECT * FROM notifications WHERE need_id = $1 ORDER BY created_at DESC",
        )
        .bind(need_id)
        .fetch_all(pool)
        .await?;

        Ok(notifications)
    }
}
