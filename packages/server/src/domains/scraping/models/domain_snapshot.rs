use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{DomainId, MemberId};

pub type DomainSnapshotId = Uuid;
pub type PageSnapshotId = Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DomainSnapshot {
    pub id: DomainSnapshotId,
    pub domain_id: Uuid, // Raw UUID for sqlx compatibility
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

impl DomainSnapshot {
    /// Get domain_id as typed ID
    pub fn get_domain_id(&self) -> DomainId {
        DomainId::from_uuid(self.domain_id)
    }

    /// Get submitted_by as typed ID
    pub fn get_submitted_by(&self) -> Option<MemberId> {
        self.submitted_by.map(MemberId::from_uuid)
    }


    /// Create or update a domain snapshot (doesn't scrape yet)
    pub async fn upsert(
        pool: &PgPool,
        domain_id: DomainId,
        page_url: String,
        submitted_by: Option<MemberId>,
    ) -> Result<Self> {
        let domain_uuid = domain_id.into_uuid();
        let submitted_by_uuid = submitted_by.map(|id| id.into_uuid());

        sqlx::query_as!(
            DomainSnapshot,
            r#"
            INSERT INTO domain_snapshots (domain_id, page_url, submitted_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (domain_id, page_url) DO UPDATE
            SET updated_at = NOW()
            RETURNING *
            "#,
            domain_uuid,
            page_url,
            submitted_by_uuid
        )
        .fetch_one(pool)
        .await
        .context("Failed to upsert domain snapshot")
    }

    /// Find domain snapshot by ID
    pub async fn find_by_id(pool: &PgPool, id: DomainSnapshotId) -> Result<Self> {
        sqlx::query_as!(
            DomainSnapshot,
            r#"SELECT * FROM domain_snapshots WHERE id = $1"#,
            id
        )
        .fetch_one(pool)
        .await
        .context("Domain snapshot not found")
    }

    /// Find all pending snapshots for approved domains
    pub async fn find_pending_for_approved_domains(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as!(
            DomainSnapshot,
            r#"
            SELECT ds.*
            FROM domain_snapshots ds
            INNER JOIN domains d ON ds.domain_id = d.id
            WHERE d.status = 'approved'
            AND ds.scrape_status = 'pending'
            ORDER BY ds.submitted_at ASC
            "#
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch pending domain snapshots")
    }

    /// Find all snapshots for a domain
    pub async fn find_by_domain(pool: &PgPool, domain_id: DomainId) -> Result<Vec<Self>> {
        let domain_uuid = domain_id.into_uuid();

        sqlx::query_as!(
            DomainSnapshot,
            r#"
            SELECT * FROM domain_snapshots
            WHERE domain_id = $1
            ORDER BY submitted_at DESC
            "#,
            domain_uuid
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch domain snapshots")
    }

    /// Link to a page snapshot after successful scrape
    pub async fn link_snapshot(&self, pool: &PgPool, snapshot_id: PageSnapshotId) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE domain_snapshots
            SET page_snapshot_id = $1,
                scrape_status = 'scraped',
                last_scraped_at = NOW(),
                scrape_error = NULL,
                updated_at = NOW()
            WHERE id = $2
            "#,
            snapshot_id,
            self.id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark as failed with error
    pub async fn mark_failed(&self, pool: &PgPool, error: String) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE domain_snapshots
            SET scrape_status = 'failed',
                scrape_error = $1,
                updated_at = NOW()
            WHERE id = $2
            "#,
            error,
            self.id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
