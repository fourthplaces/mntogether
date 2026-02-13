use anyhow::Result;
use sqlx::PgPool;

/// Reference record for zip code lat/lng lookups and proximity search
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ZipCode {
    pub zip_code: String,
    pub city: String,
    pub state: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl ZipCode {
    pub async fn find_by_code(zip: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM zip_codes WHERE zip_code = $1")
            .bind(zip)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_city(city: &str, state: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM zip_codes WHERE LOWER(city) = LOWER($1) AND state = $2 ORDER BY zip_code",
        )
        .bind(city)
        .bind(state)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
