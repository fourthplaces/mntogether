use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::OrganizationId;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    pub async fn create(
        name: &str,
        description: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO organizations (name, description) VALUES ($1, $2) RETURNING *",
        )
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM organizations ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update(
        id: OrganizationId,
        name: &str,
        description: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE organizations SET name = $2, description = $3, updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: OrganizationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
