use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typed_builder::TypedBuilder;

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

#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct CreateLocation<'a> {
    pub location_type: &'a str,
    #[builder(default)]
    pub name: Option<&'a str>,
    #[builder(default)]
    pub address_line_1: Option<&'a str>,
    #[builder(default)]
    pub city: Option<&'a str>,
    #[builder(default)]
    pub state: Option<&'a str>,
    #[builder(default)]
    pub postal_code: Option<&'a str>,
    #[builder(default)]
    pub latitude: Option<f64>,
    #[builder(default)]
    pub longitude: Option<f64>,
}

impl Location {
    pub async fn find_by_id(id: LocationId, pool: &PgPool) -> Result<Self> {
        let location = sqlx::query_as::<_, Self>("SELECT * FROM locations WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(location)
    }

    pub async fn create(params: &CreateLocation<'_>, pool: &PgPool) -> Result<Self> {
        let location = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO locations (name, address_line_1, city, state, postal_code, latitude, longitude, location_type)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(params.name)
        .bind(params.address_line_1)
        .bind(params.city)
        .bind(params.state)
        .bind(params.postal_code)
        .bind(params.latitude)
        .bind(params.longitude)
        .bind(params.location_type)
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
            let existing =
                sqlx::query_as::<_, Self>("SELECT * FROM locations WHERE postal_code = $1 LIMIT 1")
                    .bind(postal_code)
                    .fetch_optional(pool)
                    .await?;

            if let Some(loc) = existing {
                return Ok(loc);
            }
        }

        // Look up lat/lng from zip_codes reference table
        let (lat, lng) = if !postal_code.is_empty() {
            let coords: Option<(f64, f64)> =
                sqlx::query_as("SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1")
                    .bind(postal_code)
                    .fetch_optional(pool)
                    .await?;
            coords
                .map(|(la, lo)| (Some(la), Some(lo)))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        Self::create(
            &CreateLocation::builder()
                .location_type("physical")
                .address_line_1(address)
                .city(city)
                .state(state)
                .postal_code(zip)
                .latitude(lat)
                .longitude(lng)
                .build(),
            pool,
        )
        .await
    }
}
