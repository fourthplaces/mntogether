use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::LocationId;

/// Physical, virtual, or postal location where services are delivered (HSDS-aligned)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Location {
    pub id: LocationId,
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

    pub async fn create(
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
            INSERT INTO locations (name, address_line_1, city, state, postal_code, latitude, longitude, location_type)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
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

    /// Find or create a location from AI-extracted structured data.
    ///
    /// Looks up the zip code to get lat/lng, then upserts by postal_code.
    pub async fn find_or_create_from_extraction(
        city: Option<&str>,
        state: Option<&str>,
        zip: Option<&str>,
        address: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let postal_code = zip.unwrap_or_default();

        // Try to find existing location by postal_code
        if !postal_code.is_empty() {
            let existing = sqlx::query_as::<_, Self>(
                "SELECT * FROM locations WHERE postal_code = $1 LIMIT 1",
            )
            .bind(postal_code)
            .fetch_optional(pool)
            .await?;

            if let Some(loc) = existing {
                return Ok(loc);
            }
        }

        // Look up lat/lng from zip_codes reference table
        let (lat, lng) = if !postal_code.is_empty() {
            let coords: Option<(f64, f64)> = sqlx::query_as(
                "SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1",
            )
            .bind(postal_code)
            .fetch_optional(pool)
            .await?;
            coords.map(|(la, lo)| (Some(la), Some(lo))).unwrap_or((None, None))
        } else {
            (None, None)
        };

        Self::create(
            None,
            address,
            city,
            state,
            zip,
            lat,
            lng,
            "physical",
            pool,
        )
        .await
    }
}
