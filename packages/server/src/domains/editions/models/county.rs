use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Reference record for a Minnesota county.
///
/// Normally there are 87 rows (every real MN county). Synthetic rows
/// with `is_pseudo = true` also live here — right now that's the
/// "Statewide" pseudo county used to compose a statewide-tagged
/// broadsheet for the public home page default.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct County {
    pub id: Uuid,
    pub fips_code: String,
    pub name: String,
    pub state: String,
    pub latitude: f64,
    pub longitude: f64,
    pub created_at: DateTime<Utc>,
    /// Editorial weight target for this county's weekly broadsheet.
    /// Sum of post weights (heavy=3, medium=2, light=1). Root Signal aims for
    /// this total; the layout engine flexes ±30% based on actual pool.
    pub target_content_weight: i32,
    /// Synthetic row (e.g. "Statewide") rather than a real MN county.
    /// Layout engine and editorial workflows treat pseudo counties as
    /// first-class for generation but exclude them from "N of 87"-style
    /// coverage roll-ups.
    pub is_pseudo: bool,
}

impl County {
    /// Load all counties (real + pseudo), pseudo first so "Statewide"
    /// sits at the top of any UI list that uses the default order.
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM counties ORDER BY is_pseudo DESC, name ASC"
        )
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find a county by its primary key.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM counties WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find a county by its FIPS code (e.g. '27053' for Hennepin).
    pub async fn find_by_fips(fips_code: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM counties WHERE fips_code = $1")
            .bind(fips_code)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find a county by name (case-insensitive).
    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM counties WHERE LOWER(name) = LOWER($1)")
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Update the editorial weight target for a county.
    pub async fn update_target_content_weight(
        id: Uuid,
        weight: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE counties SET target_content_weight = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(weight)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Insert or return an existing county row by FIPS code. Seed / test
    /// fixtures use this; in production the counties are loaded once by
    /// migration 000174 and never added to.
    pub async fn upsert(
        fips_code: &str,
        name: &str,
        state: &str,
        latitude: f64,
        longitude: f64,
        target_content_weight: i32,
        is_pseudo: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO counties (
                fips_code, name, state, latitude, longitude,
                target_content_weight, is_pseudo
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (fips_code) DO UPDATE SET
                name = EXCLUDED.name,
                state = EXCLUDED.state,
                latitude = EXCLUDED.latitude,
                longitude = EXCLUDED.longitude,
                target_content_weight = EXCLUDED.target_content_weight,
                is_pseudo = EXCLUDED.is_pseudo
            RETURNING *
            "#,
        )
        .bind(fips_code)
        .bind(name)
        .bind(state)
        .bind(latitude)
        .bind(longitude)
        .bind(target_content_weight)
        .bind(is_pseudo)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
