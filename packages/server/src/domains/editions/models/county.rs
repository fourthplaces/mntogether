use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Reference record for a Minnesota county (87 total).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct County {
    pub id: Uuid,
    pub fips_code: String,
    pub name: String,
    pub state: String,
    pub latitude: f64,
    pub longitude: f64,
    pub created_at: DateTime<Utc>,
}

impl County {
    /// Load all counties, ordered by name.
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM counties ORDER BY name ASC")
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
}
