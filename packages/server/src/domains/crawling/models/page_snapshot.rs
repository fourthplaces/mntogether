//! Page Snapshot - cached raw page content from crawling
//!
//! Snapshots are deduplicated by content hash to avoid storing duplicate content.
//!
//! TODO(migration): Remove this module after verifying no production callers.
//! See `domains/crawling/MIGRATION.md` for removal checklist.
//!
//! # Deprecation Notice
//!
//! This module is deprecated. Use the extraction library's `CachedPage` instead.
//!
//! **Migration path:**
//! - For **storing pages**: Use `ExtractionService::ingest()` which writes to `extraction_pages`
//! - For **reading pages**: Use `extraction::PageCache::get_page()` on `PostgresStore`
//! - For **page context**: Use extraction library's `CachedPage` type
//!
//! **Schema differences:**
//!
//! | page_snapshots (old) | extraction_pages (new) |
//! |---------------------|------------------------|
//! | UUID primary key | URL as primary key |
//! | html + markdown columns | Single content column |
//! | content_hash BYTEA | content_hash TEXT (hex) |
//! | fetched_via column | In metadata |
//! | crawled_at | fetched_at |
//! | - | site_url, title, http_headers |
//!
//! The new path via `ingest_website()` stores pages directly in `extraction_pages`.

#![allow(deprecated)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[deprecated(since = "0.1.0", note = "Use extraction library's CachedPage from PostgresStore instead")]
pub type PageSnapshotId = Uuid;

/// Cached raw page content from crawling.
///
/// **Deprecated:** Use `extraction::CachedPage` instead.
/// See module documentation for migration guide.
#[deprecated(since = "0.1.0", note = "Use extraction library's CachedPage via ExtractionService instead")]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PageSnapshot {
    pub id: PageSnapshotId,
    pub url: String,
    pub content_hash: Vec<u8>,
    pub html: String,
    pub markdown: Option<String>,
    pub fetched_via: String,
    pub metadata: serde_json::Value,
    pub crawled_at: DateTime<Utc>,
    pub listings_extracted_count: Option<i32>,
    pub extraction_completed_at: Option<DateTime<Utc>>,
    pub extraction_status: Option<String>,
}

impl PageSnapshot {
    /// Create or find existing page snapshot (deduplication via content_hash)
    /// Returns (snapshot, is_new_snapshot)
    pub async fn upsert(
        pool: &PgPool,
        url: String,
        html: String,
        markdown: Option<String>,
        fetched_via: String,
    ) -> Result<(Self, bool)> {
        // Compute content hash
        let mut hasher = Sha256::new();
        hasher.update(html.as_bytes());
        let content_hash = hasher.finalize().to_vec();

        // Try to find existing snapshot with same URL and content
        let existing: Option<Self> = sqlx::query_as::<_, Self>(
            "SELECT * FROM page_snapshots
             WHERE url = $1 AND content_hash = $2",
        )
        .bind(&url)
        .bind(&content_hash)
        .fetch_optional(pool)
        .await
        .context("Failed to check for existing page snapshot")?;

        if let Some(snapshot) = existing {
            tracing::info!(
                url = %url,
                snapshot_id = %snapshot.id,
                "Found existing page snapshot with matching content"
            );
            return Ok((snapshot, false));
        }

        // Create new snapshot
        let id = PageSnapshotId::new_v4();
        let snapshot: Self = sqlx::query_as::<_, Self>(
            "INSERT INTO page_snapshots (
                id, url, content_hash, html, markdown, fetched_via,
                metadata, crawled_at, extraction_status
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), 'pending')
             RETURNING *",
        )
        .bind(id)
        .bind(&url)
        .bind(&content_hash)
        .bind(&html)
        .bind(&markdown)
        .bind(&fetched_via)
        .bind(serde_json::json!({}))
        .fetch_one(pool)
        .await
        .context("Failed to create page snapshot")?;

        tracing::info!(
            url = %url,
            snapshot_id = %snapshot.id,
            content_length = html.len(),
            "Created new page snapshot"
        );

        Ok((snapshot, true))
    }

    /// Find page snapshot by ID
    pub async fn find_by_id(pool: &PgPool, id: PageSnapshotId) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM page_snapshots
             WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .context("Page snapshot not found")
    }

    /// Mark extraction as started
    pub async fn mark_extraction_started(pool: &PgPool, id: PageSnapshotId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE page_snapshots
            SET extraction_status = 'processing'
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark extraction as completed
    pub async fn mark_extraction_completed(pool: &PgPool, id: PageSnapshotId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE page_snapshots
            SET extraction_status = 'completed',
                extraction_completed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark extraction as failed
    pub async fn mark_extraction_failed(pool: &PgPool, id: PageSnapshotId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE page_snapshots
            SET extraction_status = 'failed'
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update extraction status with count
    pub async fn update_extraction_status(
        pool: &PgPool,
        id: PageSnapshotId,
        listings_count: i32,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE page_snapshots
            SET
                extraction_status = $2,
                listings_extracted_count = $3,
                extraction_completed_at = CASE WHEN $2 = 'completed' THEN NOW() ELSE extraction_completed_at END
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(listings_count)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update page content after re-scraping
    /// Resets extraction status to pending so posts can be regenerated
    pub async fn update_content(
        pool: &PgPool,
        id: PageSnapshotId,
        html: String,
        markdown: Option<String>,
        fetched_via: String,
    ) -> Result<Self> {
        // Compute new content hash
        let mut hasher = Sha256::new();
        hasher.update(html.as_bytes());
        let content_hash = hasher.finalize().to_vec();

        let snapshot = sqlx::query_as::<_, Self>(
            r#"
            UPDATE page_snapshots
            SET
                html = $2,
                markdown = $3,
                content_hash = $4,
                fetched_via = $5,
                crawled_at = NOW(),
                extraction_status = 'pending',
                extraction_completed_at = NULL
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&html)
        .bind(&markdown)
        .bind(&content_hash)
        .bind(&fetched_via)
        .fetch_one(pool)
        .await
        .context("Failed to update page snapshot content")?;

        tracing::info!(
            snapshot_id = %id,
            url = %snapshot.url,
            content_length = html.len(),
            "Updated page snapshot content"
        );

        Ok(snapshot)
    }
}
