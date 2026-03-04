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

}
