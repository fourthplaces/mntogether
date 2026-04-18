//! Organization Links — first-class storage for "this org is on Instagram
//! at X, on Facebook at Y", replacing the old Platform tag kind.
//!
//! Each row is `(organization_id, platform, url, is_public, display_order)`.
//! The `platform` slug matches a row in `tags` where `kind = 'platform'`, so
//! the picker UI can render the same display name + emoji + color it used to
//! show when platforms were tags. See migration 232 for the design rationale.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{OrganizationId, OrganizationLinkId};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationLink {
    pub id: OrganizationLinkId,
    pub organization_id: OrganizationId,
    pub platform: String,
    pub url: String,
    pub is_public: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Denormalized display fields joined from `tags` where `kind='platform'`
    /// and `value=platform`. NULL if the slug doesn't resolve to a known
    /// platform (e.g. legacy data or a slug we haven't seeded).
    pub platform_label: Option<String>,
    pub platform_emoji: Option<String>,
    pub platform_color: Option<String>,
}

impl OrganizationLink {
    /// All links for a given org, ordered for display. Used by both the
    /// admin editor (sees all) and the public resolver (which filters
    /// `is_public = true` before rendering).
    pub async fn find_by_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT ol.*,
                   t.display_name AS platform_label,
                   t.emoji        AS platform_emoji,
                   t.color        AS platform_color
            FROM organization_links ol
            LEFT JOIN tags t
                   ON t.kind = 'platform' AND t.value = ol.platform
            WHERE ol.organization_id = $1
            ORDER BY ol.display_order ASC, ol.created_at ASC
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: OrganizationLinkId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT ol.*,
                   t.display_name AS platform_label,
                   t.emoji        AS platform_emoji,
                   t.color        AS platform_color
            FROM organization_links ol
            LEFT JOIN tags t
                   ON t.kind = 'platform' AND t.value = ol.platform
            WHERE ol.id = $1
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Insert a new link. `display_order` defaults to the next position
    /// after the largest existing order for this org, so new links append
    /// to the bottom of the editor list.
    pub async fn create(
        organization_id: OrganizationId,
        platform: &str,
        url: &str,
        is_public: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        // Insert the row, then re-read via find_by_id so the returned record
        // includes the joined platform_label/emoji/color (RETURNING * can't
        // emit columns that don't exist on the base table).
        let id: OrganizationLinkId = sqlx::query_scalar(
            r#"
            INSERT INTO organization_links
              (organization_id, platform, url, is_public, display_order)
            VALUES
              ($1, $2, $3, $4,
               COALESCE(
                 (SELECT MAX(display_order) + 1
                  FROM organization_links
                  WHERE organization_id = $1),
                 0))
            RETURNING id
            "#,
        )
        .bind(organization_id)
        .bind(platform)
        .bind(url)
        .bind(is_public)
        .fetch_one(pool)
        .await?;

        Self::find_by_id(id, pool).await
    }

    /// Update an existing link. All fields rewritten atomically.
    pub async fn update(
        id: OrganizationLinkId,
        platform: &str,
        url: &str,
        is_public: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query(
            r#"
            UPDATE organization_links
            SET platform = $2,
                url = $3,
                is_public = $4,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(platform)
        .bind(url)
        .bind(is_public)
        .execute(pool)
        .await?;

        Self::find_by_id(id, pool).await
    }

    pub async fn delete(id: OrganizationLinkId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organization_links WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Rewrite `display_order` for a set of ids in one transaction. The
    /// caller passes ids in the intended order; this writes 0..n-1.
    /// Silently skips ids that don't belong to the given org — prevents
    /// a caller from reordering rows across orgs.
    pub async fn reorder(
        organization_id: OrganizationId,
        ordered_ids: &[OrganizationLinkId],
        pool: &PgPool,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;
        for (position, id) in ordered_ids.iter().enumerate() {
            sqlx::query(
                r#"
                UPDATE organization_links
                SET display_order = $3, updated_at = NOW()
                WHERE id = $1 AND organization_id = $2
                "#,
            )
            .bind(id)
            .bind(organization_id)
            .bind(position as i32)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
