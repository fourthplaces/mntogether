use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{OrganizationId, SocialProfileId};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SocialProfile {
    pub id: SocialProfileId,
    pub organization_id: OrganizationId,
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SocialProfile {
    pub async fn create(
        organization_id: OrganizationId,
        platform: &str,
        handle: &str,
        url: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO social_profiles (organization_id, platform, handle, url)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(organization_id)
        .bind(platform)
        .bind(handle)
        .bind(url)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find or create a social profile by platform + handle.
    /// If already exists, returns the existing one without modification.
    pub async fn find_or_create(
        organization_id: OrganizationId,
        platform: &str,
        handle: &str,
        url: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        // Try insert, on conflict return existing
        let result = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO social_profiles (organization_id, platform, handle, url)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (platform, handle) DO UPDATE
            SET platform = EXCLUDED.platform
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(platform)
        .bind(handle)
        .bind(url)
        .fetch_one(pool)
        .await?;
        Ok(result)
    }

    pub async fn find_by_id(id: SocialProfileId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM social_profiles WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM social_profiles WHERE organization_id = $1 ORDER BY platform, handle",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM social_profiles
             WHERE active = true
             AND (last_scraped_at IS NULL
                  OR last_scraped_at < now() - (scrape_frequency_hours || ' hours')::interval)
             ORDER BY last_scraped_at ASC NULLS FIRST",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_last_scraped(id: SocialProfileId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "UPDATE social_profiles SET last_scraped_at = now(), updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(id: SocialProfileId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM social_profiles WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
