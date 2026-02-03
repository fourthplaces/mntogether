//! Page Summary - cached AI-generated summaries of page content
//!
//! Summaries are linked to page snapshots via content hash for cache invalidation.
//! If a page's content changes (new hash), a new summary will be generated.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::PageSnapshotId;

pub type PageSummaryId = Uuid;

/// Cached AI-extracted content from a page
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PageSummary {
    pub id: PageSummaryId,
    pub page_snapshot_id: PageSnapshotId,
    pub content_hash: String, // Hex-encoded hash for cache key
    pub content: String,      // Extracted meaningful content
    pub created_at: DateTime<Utc>,
}

impl PageSummary {
    /// Find a cached summary by content hash
    pub async fn find_by_hash(hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM page_summaries WHERE content_hash = $1")
            .bind(hash)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find summary for a specific page snapshot
    pub async fn find_by_snapshot_id(
        snapshot_id: PageSnapshotId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM page_summaries WHERE page_snapshot_id = $1")
            .bind(snapshot_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Create a new page summary (or return existing on hash conflict)
    pub async fn create(
        snapshot_id: PageSnapshotId,
        content_hash: &str,
        content: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO page_summaries (page_snapshot_id, content_hash, content)
            VALUES ($1, $2, $3)
            ON CONFLICT (content_hash) DO UPDATE SET content_hash = EXCLUDED.content_hash
            RETURNING *
            "#,
        )
        .bind(snapshot_id)
        .bind(content_hash)
        .bind(content)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete a summary
    pub async fn delete(id: PageSummaryId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM page_summaries WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all summaries for a snapshot
    pub async fn delete_for_snapshot(snapshot_id: PageSnapshotId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM page_summaries WHERE page_snapshot_id = $1")
            .bind(snapshot_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Convert raw content hash bytes to hex string
pub fn hash_to_hex(hash: &[u8]) -> String {
    hex::encode(hash)
}
