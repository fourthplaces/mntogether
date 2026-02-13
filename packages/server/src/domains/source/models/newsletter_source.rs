use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{NewsletterSourceId, SourceId};

/// Newsletter-specific source details (1:1 extension of sources)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NewsletterSource {
    pub id: NewsletterSourceId,
    pub source_id: SourceId,
    pub ingest_email: String,
    pub signup_form_url: String,
    pub subscription_status: String,
    pub confirmation_link: Option<String>,
    pub confirmation_email_received_at: Option<DateTime<Utc>>,
    pub expected_sender_domain: Option<String>,
    pub last_newsletter_received_at: Option<DateTime<Utc>>,
    pub newsletters_received_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NewsletterSource {
    pub async fn find_by_source_id(source_id: SourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM newsletter_sources WHERE source_id = $1")
            .bind(source_id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_source_id_optional(
        source_id: SourceId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM newsletter_sources WHERE source_id = $1")
            .bind(source_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_ingest_email(email: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM newsletter_sources WHERE ingest_email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_status(status: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM newsletter_sources WHERE subscription_status = $1 ORDER BY created_at DESC",
        )
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(
        source_id: SourceId,
        ingest_email: &str,
        signup_form_url: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO newsletter_sources (source_id, ingest_email, signup_form_url)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(source_id)
        .bind(ingest_email)
        .bind(signup_form_url)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_status(id: NewsletterSourceId, status: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE newsletter_sources SET subscription_status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn set_confirmation_link(
        id: NewsletterSourceId,
        confirmation_link: &str,
        sender_domain: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE newsletter_sources
            SET confirmation_link = $2,
                expected_sender_domain = $3,
                confirmation_email_received_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(confirmation_link)
        .bind(sender_domain)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn record_newsletter_received(id: NewsletterSourceId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE newsletter_sources
            SET newsletters_received_count = newsletters_received_count + 1,
                last_newsletter_received_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Generate a unique ingest email address for a new subscription
    pub fn generate_ingest_email() -> String {
        let id = Uuid::new_v4();
        format!("{}@ingest.mntogether.org", id)
    }
}
