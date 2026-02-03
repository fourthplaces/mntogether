//! Page Extraction - AI-extracted content from page snapshots
//!
//! Generic storage for different types of AI extractions (summary, posts, contacts, etc.)
//! with versioning and model tracking.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::PageSnapshotId;

pub type PageExtractionId = Uuid;

/// Known extraction types (not exhaustive - new types can be added without code changes)
pub mod extraction_types {
    pub const SUMMARY: &str = "summary";
    pub const POSTS: &str = "posts";
    pub const CONTACTS: &str = "contacts";
    pub const HOURS: &str = "hours";
    pub const EVENTS: &str = "events";
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PageExtraction {
    pub id: PageExtractionId,
    pub page_snapshot_id: PageSnapshotId,
    pub extraction_type: String,
    pub content: serde_json::Value,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
    pub tokens_used: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Content structure for summary extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryContent {
    pub text: String,
    #[serde(default)]
    pub key_services: Vec<String>,
    #[serde(default)]
    pub target_audience: Option<String>,
}

/// Content structure for contacts extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsContent {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub intake_form_url: Option<String>,
    pub website: Option<String>,
}

/// Content structure for hours extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoursContent {
    pub monday: Option<String>,
    pub tuesday: Option<String>,
    pub wednesday: Option<String>,
    pub thursday: Option<String>,
    pub friday: Option<String>,
    pub saturday: Option<String>,
    pub sunday: Option<String>,
    pub notes: Option<String>,
}

impl PageExtraction {
    /// Create a new extraction, marking any existing current extraction as not current
    pub async fn create(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
        extraction_type: &str,
        content: serde_json::Value,
        model: Option<String>,
        prompt_version: Option<String>,
        tokens_used: Option<i32>,
    ) -> Result<Self> {
        // Mark existing current extraction as not current
        sqlx::query(
            r#"
            UPDATE page_extractions
            SET is_current = FALSE
            WHERE page_snapshot_id = $1
              AND extraction_type = $2
              AND is_current = TRUE
            "#,
        )
        .bind(page_snapshot_id)
        .bind(extraction_type)
        .execute(pool)
        .await
        .context("Failed to mark previous extraction as not current")?;

        // Insert new extraction
        let extraction = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO page_extractions (
                page_snapshot_id, extraction_type, content, model,
                prompt_version, tokens_used, is_current
            )
            VALUES ($1, $2, $3, $4, $5, $6, TRUE)
            RETURNING *
            "#,
        )
        .bind(page_snapshot_id)
        .bind(extraction_type)
        .bind(&content)
        .bind(&model)
        .bind(&prompt_version)
        .bind(tokens_used)
        .fetch_one(pool)
        .await
        .context("Failed to create page extraction")?;

        tracing::info!(
            extraction_id = %extraction.id,
            page_snapshot_id = %page_snapshot_id,
            extraction_type = %extraction_type,
            model = ?model,
            "Created new page extraction"
        );

        Ok(extraction)
    }

    /// Find the current extraction for a page and type
    pub async fn find_current(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
        extraction_type: &str,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM page_extractions
            WHERE page_snapshot_id = $1
              AND extraction_type = $2
              AND is_current = TRUE
            "#,
        )
        .bind(page_snapshot_id)
        .bind(extraction_type)
        .fetch_optional(pool)
        .await
        .context("Failed to find current extraction")
    }

    /// Find all current extractions for a page
    pub async fn find_all_current(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM page_extractions
            WHERE page_snapshot_id = $1
              AND is_current = TRUE
            ORDER BY extraction_type
            "#,
        )
        .bind(page_snapshot_id)
        .fetch_all(pool)
        .await
        .context("Failed to find current extractions")
    }

    /// Find extraction history for a page and type
    pub async fn find_history(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
        extraction_type: &str,
        limit: i64,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM page_extractions
            WHERE page_snapshot_id = $1
              AND extraction_type = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(page_snapshot_id)
        .bind(extraction_type)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to find extraction history")
    }

    /// Get the content as a typed struct (for summary)
    pub fn content_as_summary(&self) -> Result<SummaryContent> {
        serde_json::from_value(self.content.clone()).context("Failed to parse summary content")
    }

    /// Get the content as a typed struct (for contacts)
    pub fn content_as_contacts(&self) -> Result<ContactsContent> {
        serde_json::from_value(self.content.clone()).context("Failed to parse contacts content")
    }

    /// Get the content as a typed struct (for hours)
    pub fn content_as_hours(&self) -> Result<HoursContent> {
        serde_json::from_value(self.content.clone()).context("Failed to parse hours content")
    }
}

// Convenience functions for creating typed extractions
impl PageExtraction {
    /// Create a summary extraction
    pub async fn create_summary(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
        summary: &SummaryContent,
        model: Option<String>,
        prompt_version: Option<String>,
        tokens_used: Option<i32>,
    ) -> Result<Self> {
        let content =
            serde_json::to_value(summary).context("Failed to serialize summary content")?;

        Self::create(
            pool,
            page_snapshot_id,
            extraction_types::SUMMARY,
            content,
            model,
            prompt_version,
            tokens_used,
        )
        .await
    }

    /// Create a contacts extraction
    pub async fn create_contacts(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
        contacts: &ContactsContent,
        model: Option<String>,
        prompt_version: Option<String>,
        tokens_used: Option<i32>,
    ) -> Result<Self> {
        let content =
            serde_json::to_value(contacts).context("Failed to serialize contacts content")?;

        Self::create(
            pool,
            page_snapshot_id,
            extraction_types::CONTACTS,
            content,
            model,
            prompt_version,
            tokens_used,
        )
        .await
    }
}
