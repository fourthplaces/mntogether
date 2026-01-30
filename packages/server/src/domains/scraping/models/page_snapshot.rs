use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub type PageSnapshotId = Uuid;

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
        let existing = sqlx::query_as!(
            PageSnapshot,
            r#"
            SELECT
                id,
                url,
                content_hash,
                html,
                markdown,
                fetched_via,
                metadata,
                crawled_at,
                listings_extracted_count,
                extraction_completed_at,
                extraction_status
            FROM page_snapshots
            WHERE url = $1 AND content_hash = $2
            "#,
            url,
            content_hash
        )
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
        let snapshot = sqlx::query_as!(
            PageSnapshot,
            r#"
            INSERT INTO page_snapshots (
                id, url, content_hash, html, markdown, fetched_via,
                metadata, crawled_at, extraction_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), 'pending')
            RETURNING
                id,
                url,
                content_hash,
                html,
                markdown,
                fetched_via,
                metadata,
                crawled_at,
                listings_extracted_count,
                extraction_completed_at,
                extraction_status
            "#,
            id,
            url,
            content_hash,
            html,
            markdown,
            fetched_via,
            serde_json::json!({})
        )
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
        sqlx::query_as!(
            PageSnapshot,
            r#"
            SELECT
                id,
                url,
                content_hash,
                html,
                markdown,
                fetched_via,
                metadata,
                crawled_at,
                listings_extracted_count,
                extraction_completed_at,
                extraction_status
            FROM page_snapshots
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(pool)
        .await
        .context("Page snapshot not found")
    }

    /// Mark extraction as started
    pub async fn mark_extraction_started(pool: &PgPool, id: PageSnapshotId) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE page_snapshots
            SET extraction_status = 'processing'
            WHERE id = $1
            "#,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark extraction as completed
    pub async fn mark_extraction_completed(pool: &PgPool, id: PageSnapshotId) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE page_snapshots
            SET extraction_status = 'completed',
                extraction_completed_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark extraction as failed
    pub async fn mark_extraction_failed(pool: &PgPool, id: PageSnapshotId) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE page_snapshots
            SET extraction_status = 'failed'
            WHERE id = $1
            "#,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
