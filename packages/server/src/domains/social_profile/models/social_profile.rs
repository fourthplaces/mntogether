use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{OrganizationId, SocialProfileId};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SocialProfile {
    pub id: SocialProfileId,
    pub organization_id: Option<OrganizationId>,
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// SQL Queries - target: sources s JOIN social_sources ss ON ss.source_id = s.id
// =============================================================================

impl SocialProfile {
    /// Base SELECT query for SocialProfile from joined sources + social_sources.
    fn base_query(suffix: &str) -> String {
        format!(
            "SELECT s.id, s.organization_id, ss.source_type AS platform, ss.handle, s.url, \
             s.scrape_frequency_hours, s.last_scraped_at, s.active, s.created_at, s.updated_at \
             FROM sources s JOIN social_sources ss ON ss.source_id = s.id {}",
            suffix
        )
    }

    pub async fn create(
        organization_id: OrganizationId,
        platform: &str,
        handle: &str,
        url: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        // Create in transaction: sources + social_sources
        let mut tx = pool.begin().await?;

        let source_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO sources (source_type, url, organization_id, status, active)
             VALUES ($1, $2, $3, 'approved', true)
             RETURNING id",
        )
        .bind(platform)
        .bind(url)
        .bind(organization_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO social_sources (source_id, source_type, handle)
             VALUES ($1, $2, $3)",
        )
        .bind(source_id)
        .bind(platform)
        .bind(handle)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Self::find_by_id(SocialProfileId::from_uuid(source_id), pool).await
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
        // Check if exists
        let q = Self::base_query("WHERE ss.source_type = $1 AND ss.handle = $2");
        if let Some(existing) = sqlx::query_as::<_, Self>(&q)
            .bind(platform)
            .bind(handle)
            .fetch_optional(pool)
            .await?
        {
            return Ok(existing);
        }

        Self::create(organization_id, platform, handle, url, pool).await
    }

    pub async fn find_by_id(id: SocialProfileId, pool: &PgPool) -> Result<Self> {
        let q = Self::base_query("WHERE s.id = $1");
        sqlx::query_as::<_, Self>(&q)
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let q = Self::base_query("WHERE s.organization_id = $1 ORDER BY ss.source_type, ss.handle");
        sqlx::query_as::<_, Self>(&q)
            .bind(organization_id)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_due_for_scraping(pool: &PgPool) -> Result<Vec<Self>> {
        let q = Self::base_query(
            r#"WHERE s.active = true
             AND (s.last_scraped_at IS NULL
                  OR s.last_scraped_at < now() - (s.scrape_frequency_hours || ' hours')::interval)
             ORDER BY s.last_scraped_at ASC NULLS FIRST"#,
        );
        sqlx::query_as::<_, Self>(&q)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_last_scraped(id: SocialProfileId, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE sources SET last_scraped_at = now(), updated_at = now() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(id: SocialProfileId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM sources WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
