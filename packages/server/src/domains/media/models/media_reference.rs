//! MediaReference — polymorphic link between a media item and the entity
//! that uses it (post hero, post person, post body image, widget, org logo).
//!
//! Every write path that touches media (upsert_post_media, upsert_post_person,
//! update_post body, update_widget data, update_organization logo) calls
//! `reconcile(...)` so the Library's usage counts stay current without needing
//! to scan JSONB blobs.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MediaReference {
    pub id: Uuid,
    pub media_id: Uuid,
    pub referenceable_type: String,
    pub referenceable_id: Uuid,
    pub field_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A resolved usage row: the media reference joined to the human-readable
/// title of the referring entity, for display in the Library detail panel.
#[derive(Debug, Clone, Serialize)]
pub struct MediaUsage {
    pub media_id: Uuid,
    pub referenceable_type: String,
    pub referenceable_id: Uuid,
    pub field_key: Option<String>,
    pub title: String,
}

/// A single desired reference, produced by write paths and passed to
/// `reconcile` to be inserted after clearing existing refs for the entity.
#[derive(Debug, Clone)]
pub struct DesiredRef {
    pub media_id: Uuid,
    pub field_key: Option<String>,
}

impl MediaReference {
    /// All references pointing at a given media row.
    pub async fn find_by_media(media_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM media_references WHERE media_id = $1 ORDER BY created_at ASC",
        )
        .bind(media_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// All references from a given entity.
    pub async fn find_by_entity(
        referenceable_type: &str,
        referenceable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let rows = sqlx::query_as::<_, Self>(
            "SELECT * FROM media_references
             WHERE referenceable_type = $1 AND referenceable_id = $2
             ORDER BY created_at ASC",
        )
        .bind(referenceable_type)
        .bind(referenceable_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Reconcile the full set of media references for an entity. Transactional:
    /// deletes existing references for (type, id), then inserts the desired
    /// set. Safe to call every write — idempotent, cheap (tiny row counts).
    ///
    /// `desired` may be empty (meaning "this entity now references no media").
    /// Duplicate (media_id, field_key) pairs in `desired` are collapsed.
    pub async fn reconcile(
        referenceable_type: &str,
        referenceable_id: Uuid,
        desired: &[DesiredRef],
        pool: &PgPool,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;

        sqlx::query(
            "DELETE FROM media_references
             WHERE referenceable_type = $1 AND referenceable_id = $2",
        )
        .bind(referenceable_type)
        .bind(referenceable_id)
        .execute(&mut *tx)
        .await?;

        for d in desired {
            // ON CONFLICT guards against duplicate (type, id, media_id, field_key)
            // tuples in the desired set.
            sqlx::query(
                "INSERT INTO media_references (media_id, referenceable_type, referenceable_id, field_key)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (media_id, referenceable_type, referenceable_id, field_key) DO NOTHING",
            )
            .bind(d.media_id)
            .bind(referenceable_type)
            .bind(referenceable_id)
            .bind(d.field_key.as_deref())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Clear all references for an entity. Called when the entity is deleted.
    pub async fn delete_by_entity(
        referenceable_type: &str,
        referenceable_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM media_references
             WHERE referenceable_type = $1 AND referenceable_id = $2",
        )
        .bind(referenceable_type)
        .bind(referenceable_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Resolve a media's usage list to human-readable entries for the UI.
    /// Joins against posts / widgets / organizations based on referenceable_type.
    pub async fn list_usage(media_id: Uuid, pool: &PgPool) -> Result<Vec<MediaUsage>> {
        // Unioned across the three entity kinds. Each branch joins to the
        // right table to produce a title. Widgets don't have a natural title
        // — use the widget_type for now (editors can rename widgets later if
        // it gets noisy).
        let rows: Vec<MediaUsage> = sqlx::query_as::<_, (Uuid, String, Uuid, Option<String>, String)>(
            r#"
            SELECT mr.media_id, mr.referenceable_type, mr.referenceable_id, mr.field_key,
                   p.title AS title
            FROM media_references mr
            JOIN posts p ON p.id = mr.referenceable_id
            WHERE mr.media_id = $1 AND mr.referenceable_type IN ('post_hero', 'post_person', 'post_body')

            UNION ALL

            SELECT mr.media_id, mr.referenceable_type, mr.referenceable_id, mr.field_key,
                   w.widget_type AS title
            FROM media_references mr
            JOIN widgets w ON w.id = mr.referenceable_id
            WHERE mr.media_id = $1 AND mr.referenceable_type = 'widget'

            UNION ALL

            SELECT mr.media_id, mr.referenceable_type, mr.referenceable_id, mr.field_key,
                   o.name AS title
            FROM media_references mr
            JOIN organizations o ON o.id = mr.referenceable_id
            WHERE mr.media_id = $1 AND mr.referenceable_type = 'organization_logo'
            "#,
        )
        .bind(media_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(media_id, rtype, rid, fkey, title)| MediaUsage {
            media_id,
            referenceable_type: rtype,
            referenceable_id: rid,
            field_key: fkey,
            title,
        })
        .collect();

        Ok(rows)
    }

    /// Count of usage references for a media item.
    pub async fn count_for_media(media_id: Uuid, pool: &PgPool) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM media_references WHERE media_id = $1",
        )
        .bind(media_id)
        .fetch_one(pool)
        .await?;
        Ok(n)
    }
}
