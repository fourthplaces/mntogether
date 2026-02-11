use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{LocatableId, LocationId};
use crate::domains::locations::models::Location;

/// Polymorphic join table linking locations to any entity (mirrors Noteable).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Locationable {
    pub id: LocatableId,
    pub location_id: LocationId,
    pub locatable_type: String,
    pub locatable_id: Uuid,
    pub is_primary: bool,
    pub notes: Option<String>,
    pub added_at: DateTime<Utc>,
}

impl Locationable {
    /// Link a location to an entity. Idempotent (ON CONFLICT DO UPDATE).
    pub async fn create(
        location_id: LocationId,
        locatable_type: &str,
        locatable_id: Uuid,
        is_primary: bool,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO locationables (location_id, locatable_type, locatable_id, is_primary, notes)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (location_id, locatable_type, locatable_id) DO UPDATE
            SET is_primary = EXCLUDED.is_primary,
                notes = COALESCE(EXCLUDED.notes, locationables.notes)
            RETURNING *
            "#,
        )
        .bind(location_id)
        .bind(locatable_type)
        .bind(locatable_id)
        .bind(is_primary)
        .bind(notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Unlink a location from an entity.
    pub async fn delete(
        location_id: LocationId,
        locatable_type: &str,
        locatable_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM locationables WHERE location_id = $1 AND locatable_type = $2 AND locatable_id = $3",
        )
        .bind(location_id)
        .bind(locatable_type)
        .bind(locatable_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find all locations for an entity (with full Location details).
    pub async fn find_for_entity(
        locatable_type: &str,
        locatable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Location>> {
        sqlx::query_as::<_, Location>(
            r#"
            SELECT l.*
            FROM locations l
            INNER JOIN locationables loc ON loc.location_id = l.id
            WHERE loc.locatable_type = $1 AND loc.locatable_id = $2
            ORDER BY loc.is_primary DESC, l.name ASC
            "#,
        )
        .bind(locatable_type)
        .bind(locatable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all locationable links for an entity.
    pub async fn find_links_for_entity(
        locatable_type: &str,
        locatable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM locationables WHERE locatable_type = $1 AND locatable_id = $2 ORDER BY is_primary DESC, added_at ASC",
        )
        .bind(locatable_type)
        .bind(locatable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all entities at a given location.
    pub async fn find_entities_at_location(
        location_id: LocationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM locationables WHERE location_id = $1 ORDER BY added_at ASC",
        )
        .bind(location_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete all location links for an entity.
    pub async fn delete_all_for_entity(
        locatable_type: &str,
        locatable_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM locationables WHERE locatable_type = $1 AND locatable_id = $2",
        )
        .bind(locatable_type)
        .bind(locatable_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
