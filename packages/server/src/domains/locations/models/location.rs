use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{LocationId, OrganizationId, PostId, PostLocationId};

/// Physical, virtual, or postal location where services are delivered (HSDS-aligned)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Location {
    pub id: LocationId,
    pub organization_id: Option<OrganizationId>,
    pub name: Option<String>,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_type: String, // 'physical', 'virtual', 'postal'
    pub accessibility_notes: Option<String>,
    pub transportation_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Location {
    pub async fn find_by_id(id: LocationId, pool: &PgPool) -> Result<Self> {
        let location = sqlx::query_as::<_, Self>("SELECT * FROM locations WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(location)
    }

    pub async fn find_by_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let locations = sqlx::query_as::<_, Self>(
            "SELECT * FROM locations WHERE organization_id = $1 ORDER BY name ASC",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;
        Ok(locations)
    }

    pub async fn create(
        organization_id: Option<OrganizationId>,
        name: Option<&str>,
        address_line_1: Option<&str>,
        city: Option<&str>,
        state: Option<&str>,
        postal_code: Option<&str>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        location_type: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let location = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO locations (organization_id, name, address_line_1, city, state, postal_code, latitude, longitude, location_type)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(name)
        .bind(address_line_1)
        .bind(city)
        .bind(state)
        .bind(postal_code)
        .bind(latitude)
        .bind(longitude)
        .bind(location_type)
        .fetch_one(pool)
        .await?;
        Ok(location)
    }

    pub async fn delete(id: LocationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM locations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Links posts to locations (HSDS service_at_location equivalent)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostLocation {
    pub id: PostLocationId,
    pub post_id: PostId,
    pub location_id: LocationId,
    pub is_primary: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PostLocation {
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        let post_locations = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_locations WHERE post_id = $1 ORDER BY is_primary DESC, created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await?;
        Ok(post_locations)
    }

    pub async fn find_by_location(location_id: LocationId, pool: &PgPool) -> Result<Vec<Self>> {
        let post_locations = sqlx::query_as::<_, Self>(
            "SELECT * FROM post_locations WHERE location_id = $1 ORDER BY created_at ASC",
        )
        .bind(location_id)
        .fetch_all(pool)
        .await?;
        Ok(post_locations)
    }

    /// Find locations for a post with full location details
    pub async fn find_locations_for_post(
        post_id: PostId,
        pool: &PgPool,
    ) -> Result<Vec<Location>> {
        let locations = sqlx::query_as::<_, Location>(
            r#"
            SELECT l.*
            FROM locations l
            INNER JOIN post_locations pl ON pl.location_id = l.id
            WHERE pl.post_id = $1
            ORDER BY pl.is_primary DESC, l.name ASC
            "#,
        )
        .bind(post_id)
        .fetch_all(pool)
        .await?;
        Ok(locations)
    }

    pub async fn create(
        post_id: PostId,
        location_id: LocationId,
        is_primary: bool,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let post_location = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_locations (post_id, location_id, is_primary, notes)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (post_id, location_id) DO UPDATE SET
                is_primary = EXCLUDED.is_primary,
                notes = COALESCE(EXCLUDED.notes, post_locations.notes)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(location_id)
        .bind(is_primary)
        .bind(notes)
        .fetch_one(pool)
        .await?;
        Ok(post_location)
    }

    pub async fn delete(post_id: PostId, location_id: LocationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_locations WHERE post_id = $1 AND location_id = $2")
            .bind(post_id)
            .bind(location_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM post_locations WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
