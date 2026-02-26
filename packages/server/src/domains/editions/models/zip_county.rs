use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Maps a zip code to a county. A zip code can belong to multiple counties
/// (rare, for zips that straddle county borders).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ZipCounty {
    pub zip_code: String,
    pub county_id: Uuid,
    pub is_primary: bool,
}

impl ZipCounty {
    /// Find all county mappings for a given zip code.
    pub async fn find_counties_for_zip(zip_code: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM zip_counties WHERE zip_code = $1 ORDER BY is_primary DESC",
        )
        .bind(zip_code)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find the primary county for a zip code.
    pub async fn find_primary_county_for_zip(
        zip_code: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM zip_counties WHERE zip_code = $1 AND is_primary = true LIMIT 1",
        )
        .bind(zip_code)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all zip codes mapped to a given county.
    pub async fn find_zips_for_county(county_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM zip_counties WHERE county_id = $1 ORDER BY zip_code ASC",
        )
        .bind(county_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
