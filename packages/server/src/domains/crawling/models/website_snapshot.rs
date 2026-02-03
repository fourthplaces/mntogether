//! Website Snapshot - links a website to its crawled page snapshots
//!
//! Tracks which pages have been crawled for each website.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PageSnapshotId;
use crate::common::{MemberId, WebsiteId};

pub type WebsiteSnapshotId = Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebsiteSnapshot {
    pub id: WebsiteSnapshotId,
    pub website_id: Uuid, // Raw UUID for sqlx compatibility
    pub page_url: String,
    pub page_snapshot_id: Option<PageSnapshotId>,
    pub submitted_by: Option<Uuid>, // Raw UUID for sqlx compatibility
    pub submitted_at: DateTime<Utc>,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub scrape_status: String,
    pub scrape_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WebsiteSnapshot {
    /// Get website_id as typed ID
    pub fn get_website_id(&self) -> WebsiteId {
        WebsiteId::from_uuid(self.website_id)
    }

    /// Get submitted_by as typed ID
    pub fn get_submitted_by(&self) -> Option<MemberId> {
        self.submitted_by.map(MemberId::from_uuid)
    }

    /// Create or update a domain snapshot (doesn't scrape yet)
    pub async fn upsert(
        pool: &PgPool,
        website_id: WebsiteId,
        page_url: String,
        submitted_by: Option<MemberId>,
    ) -> Result<Self> {
        let domain_uuid = website_id.into_uuid();
        let submitted_by_uuid = submitted_by.map(|id| id.into_uuid());

        sqlx::query_as::<_, Self>(
            "INSERT INTO website_snapshots (website_id, page_url, submitted_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (website_id, page_url) DO UPDATE
             SET updated_at = NOW()
             RETURNING *",
        )
        .bind(domain_uuid)
        .bind(page_url)
        .bind(submitted_by_uuid)
        .fetch_one(pool)
        .await
        .context("Failed to upsert domain snapshot")
    }

    /// Find domain snapshot by ID
    pub async fn find_by_id(pool: &PgPool, id: WebsiteSnapshotId) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM website_snapshots WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .context("Domain snapshot not found")
    }

    /// Find all pending snapshots for approved websites
    pub async fn find_pending_for_approved_websites(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT ds.*
             FROM website_snapshots ds
             INNER JOIN websites d ON ds.website_id = d.id
             WHERE d.status = 'approved'
             AND ds.scrape_status = 'pending'
             ORDER BY ds.submitted_at ASC",
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch pending domain snapshots")
    }

    /// Find all snapshots for a website
    pub async fn find_by_website(pool: &PgPool, website_id: WebsiteId) -> Result<Vec<Self>> {
        let website_uuid = website_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM website_snapshots
             WHERE website_id = $1
             ORDER BY submitted_at DESC",
        )
        .bind(website_uuid)
        .fetch_all(pool)
        .await
        .context("Failed to fetch website snapshots")
    }

    /// Find website snapshot by page_snapshot_id
    pub async fn find_by_page_snapshot_id(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM website_snapshots WHERE page_snapshot_id = $1")
            .bind(page_snapshot_id)
            .fetch_optional(pool)
            .await
            .context("Failed to fetch website snapshot by page_snapshot_id")
    }

    /// Link to a page snapshot after successful scrape
    pub async fn link_snapshot(&self, pool: &PgPool, snapshot_id: PageSnapshotId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE website_snapshots
            SET page_snapshot_id = $1,
                scrape_status = 'scraped',
                last_scraped_at = NOW(),
                scrape_error = NULL,
                updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(snapshot_id)
        .bind(self.id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark as failed with error
    pub async fn mark_failed(&self, pool: &PgPool, error: String) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE website_snapshots
            SET scrape_status = 'failed',
                scrape_error = $1,
                updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(error)
        .bind(self.id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
