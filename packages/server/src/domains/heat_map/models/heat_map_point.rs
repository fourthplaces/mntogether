use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::HeatMapPointId;

/// A single weighted point in the heat map snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HeatMapPoint {
    pub id: HeatMapPointId,
    pub latitude: f64,
    pub longitude: f64,
    pub weight: f64,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub generated_at: DateTime<Utc>,
}

/// Intermediate row from the heat map computation query.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HeatMapRow {
    pub latitude: f64,
    pub longitude: f64,
    pub weight: f64,
    pub entity_type: String,
    pub entity_id: Uuid,
}

impl HeatMapPoint {
    /// Return all points from the most recent snapshot.
    pub async fn find_latest(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM heat_map_points
            WHERE generated_at = (SELECT MAX(generated_at) FROM heat_map_points)
            ORDER BY weight DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Return latest points filtered by entity type.
    pub async fn find_latest_by_type(entity_type: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM heat_map_points
            WHERE generated_at = (SELECT MAX(generated_at) FROM heat_map_points)
              AND entity_type = $1
            ORDER BY weight DESC
            "#,
        )
        .bind(entity_type)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Atomically replace the snapshot: DELETE all + batch INSERT in a transaction.
    pub async fn truncate_and_insert(rows: &[HeatMapRow], pool: &PgPool) -> Result<usize> {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM heat_map_points")
            .execute(&mut *tx)
            .await?;

        let now = Utc::now();
        let mut inserted = 0;

        for row in rows {
            sqlx::query(
                r#"
                INSERT INTO heat_map_points (latitude, longitude, weight, entity_type, entity_id, generated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(row.latitude)
            .bind(row.longitude)
            .bind(row.weight)
            .bind(&row.entity_type)
            .bind(row.entity_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            inserted += 1;
        }

        tx.commit().await?;
        Ok(inserted)
    }

    /// When was the last snapshot generated?
    pub async fn latest_generated_at(pool: &PgPool) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query_as::<_, (DateTime<Utc>,)>(
            "SELECT MAX(generated_at) FROM heat_map_points",
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|(ts,)| ts))
    }
}
