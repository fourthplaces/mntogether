use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{MemberId, OrganizationId};

/// The three required checklist items before an organization can go live.
pub const CHECKLIST_KEYS: &[&str] = &[
    "obtained_consent",
    "reviewed_posts",
    "confirmed_sources",
];

pub const CHECKLIST_LABELS: &[(&str, &str)] = &[
    ("obtained_consent", "Obtained consent from the organization"),
    ("reviewed_posts", "Reviewed posts and listings for quality"),
    ("confirmed_sources", "Confirmed sources are correct"),
];

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationChecklistItem {
    pub id: Uuid,
    pub organization_id: OrganizationId,
    pub checklist_key: String,
    pub checked_by: MemberId,
    pub checked_at: DateTime<Utc>,
}

impl OrganizationChecklistItem {
    /// List all checked items for an organization.
    pub async fn find_by_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM organization_checklist_items WHERE organization_id = $1 ORDER BY checked_at ASC",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Check (toggle on) a checklist item. Returns the item if newly created.
    pub async fn check(
        organization_id: OrganizationId,
        checklist_key: &str,
        checked_by: MemberId,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO organization_checklist_items (organization_id, checklist_key, checked_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (organization_id, checklist_key) DO UPDATE
            SET checked_by = $3, checked_at = now()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(checklist_key)
        .bind(checked_by)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Uncheck (remove) a checklist item.
    pub async fn uncheck(
        organization_id: OrganizationId,
        checklist_key: &str,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM organization_checklist_items WHERE organization_id = $1 AND checklist_key = $2",
        )
        .bind(organization_id)
        .bind(checklist_key)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Reset all checklist items for an organization (e.g., on rejection).
    pub async fn reset(organization_id: OrganizationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organization_checklist_items WHERE organization_id = $1")
            .bind(organization_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Check whether all required items are checked for an organization.
    pub async fn all_checked(organization_id: OrganizationId, pool: &PgPool) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT checklist_key) FROM organization_checklist_items WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await?;
        Ok(count as usize >= CHECKLIST_KEYS.len())
    }
}
