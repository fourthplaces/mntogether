use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{TagId, TaxonomyCrosswalkId};

/// Maps internal tags to external taxonomy codes (211HSIS, Open Eligibility, NTEE)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaxonomyCrosswalk {
    pub id: TaxonomyCrosswalkId,
    pub tag_id: TagId,
    pub external_system: String, // 'open_eligibility', '211hsis', 'ntee'
    pub external_code: String,   // 'BD-1800.2000', '1102', etc.
    pub external_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TaxonomyCrosswalk {
    pub async fn find_by_id(id: TaxonomyCrosswalkId, pool: &PgPool) -> Result<Self> {
        let crosswalk =
            sqlx::query_as::<_, Self>("SELECT * FROM taxonomy_crosswalks WHERE id = $1")
                .bind(id)
                .fetch_one(pool)
                .await?;
        Ok(crosswalk)
    }

    /// Find all crosswalks for a tag
    pub async fn find_by_tag(tag_id: TagId, pool: &PgPool) -> Result<Vec<Self>> {
        let crosswalks = sqlx::query_as::<_, Self>(
            "SELECT * FROM taxonomy_crosswalks WHERE tag_id = $1 ORDER BY external_system",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await?;
        Ok(crosswalks)
    }

    /// Find tag by external system and code
    pub async fn find_by_external(
        external_system: &str,
        external_code: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let crosswalk = sqlx::query_as::<_, Self>(
            "SELECT * FROM taxonomy_crosswalks WHERE external_system = $1 AND external_code = $2",
        )
        .bind(external_system)
        .bind(external_code)
        .fetch_optional(pool)
        .await?;
        Ok(crosswalk)
    }

    pub async fn create(
        tag_id: TagId,
        external_system: &str,
        external_code: &str,
        external_name: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let crosswalk = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO taxonomy_crosswalks (tag_id, external_system, external_code, external_name)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tag_id, external_system) DO UPDATE SET
                external_code = EXCLUDED.external_code,
                external_name = COALESCE(EXCLUDED.external_name, taxonomy_crosswalks.external_name)
            RETURNING *
            "#,
        )
        .bind(tag_id)
        .bind(external_system)
        .bind(external_code)
        .bind(external_name)
        .fetch_one(pool)
        .await?;
        Ok(crosswalk)
    }

    pub async fn delete(id: TaxonomyCrosswalkId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM taxonomy_crosswalks WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
