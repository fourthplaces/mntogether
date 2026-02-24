use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{DetectedNewsletterFormId, WebsiteSourceId};

/// A newsletter signup form detected on a website during org extraction
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DetectedNewsletterForm {
    pub id: DetectedNewsletterFormId,
    pub website_source_id: WebsiteSourceId,
    pub form_url: String,
    pub form_type: String,
    pub requires_extra_fields: bool,
    pub extra_fields_detected: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DetectedNewsletterForm {
    pub async fn find_by_id(id: DetectedNewsletterFormId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM detected_newsletter_forms WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_website_source(
        website_source_id: WebsiteSourceId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM detected_newsletter_forms WHERE website_source_id = $1 ORDER BY created_at DESC",
        )
        .bind(website_source_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Upsert a detected form (deduplicate by website_source_id + form_url)
    pub async fn upsert(
        website_source_id: WebsiteSourceId,
        form_url: &str,
        form_type: &str,
        requires_extra_fields: bool,
        extra_fields: &serde_json::Value,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO detected_newsletter_forms
                (website_source_id, form_url, form_type, requires_extra_fields, extra_fields_detected)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (website_source_id, form_url)
            DO UPDATE SET
                form_type = EXCLUDED.form_type,
                requires_extra_fields = EXCLUDED.requires_extra_fields,
                extra_fields_detected = EXCLUDED.extra_fields_detected,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(website_source_id)
        .bind(form_url)
        .bind(form_type)
        .bind(requires_extra_fields)
        .bind(extra_fields)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_status(
        id: DetectedNewsletterFormId,
        status: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE detected_newsletter_forms SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
