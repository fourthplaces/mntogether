use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{PostId, ServiceAreaId};

/// Geographic coverage area for a post (HSDS service_area equivalent)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceArea {
    pub id: ServiceAreaId,
    pub post_id: PostId,
    pub area_type: String, // 'county', 'city', 'state', 'zip', 'custom'
    pub area_name: String, // 'Hennepin County', 'Minneapolis', 'MN'
    pub area_code: Option<String>, // FIPS code, ZIP code, state abbreviation
    pub created_at: DateTime<Utc>,
}

impl ServiceArea {
    pub async fn find_by_id(id: ServiceAreaId, pool: &PgPool) -> Result<Self> {
        let area = sqlx::query_as::<_, Self>("SELECT * FROM service_areas WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(area)
    }

    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        let areas = sqlx::query_as::<_, Self>(
            "SELECT * FROM service_areas WHERE post_id = $1 ORDER BY area_type, area_name",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await?;
        Ok(areas)
    }

    /// Find all posts that cover a specific area
    pub async fn find_posts_covering_area(
        area_type: &str,
        area_name: &str,
        pool: &PgPool,
    ) -> Result<Vec<PostId>> {
        let ids: Vec<(PostId,)> = sqlx::query_as(
            "SELECT post_id FROM service_areas WHERE area_type = $1 AND area_name = $2",
        )
        .bind(area_type)
        .bind(area_name)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    pub async fn create(
        post_id: PostId,
        area_type: &str,
        area_name: &str,
        area_code: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let area = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO service_areas (post_id, area_type, area_name, area_code)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(area_type)
        .bind(area_name)
        .bind(area_code)
        .fetch_one(pool)
        .await?;
        Ok(area)
    }

    pub async fn delete(id: ServiceAreaId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM service_areas WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM service_areas WHERE post_id = $1")
            .bind(post_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
