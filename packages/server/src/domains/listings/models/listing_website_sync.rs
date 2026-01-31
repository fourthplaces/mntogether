use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::entity_ids::{ListingId, WebsiteId};

/// ListingWebsiteSync - tracks when listings are seen on websites
///
/// This table decouples sync tracking from the Listing model, allowing:
/// - A listing to exist independently of any website
/// - Tracking multiple appearances of the same listing across different websites
/// - Content hash-based duplicate detection per website
/// - Temporal tracking (first_seen, last_seen, disappeared_at)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ListingWebsiteSync {
    pub id: Uuid,
    pub listing_id: Uuid, // Raw UUID for sqlx compatibility
    pub website_id: Uuid, // Raw UUID for sqlx compatibility
    pub content_hash: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub disappeared_at: Option<DateTime<Utc>>,
    pub source_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ListingWebsiteSync {
    /// Get listing_id as typed ID
    pub fn get_listing_id(&self) -> ListingId {
        ListingId::from_uuid(self.listing_id)
    }

    /// Get website_id as typed ID
    pub fn get_website_id(&self) -> WebsiteId {
        WebsiteId::from_uuid(self.website_id)
    }

    /// Find sync record by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM listing_website_sync WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find sync record by listing and website
    pub async fn find_by_listing_and_website(
        listing_id: ListingId,
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let listing_uuid = listing_id.into_uuid();
        let website_uuid = website_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM listing_website_sync WHERE listing_id = $1 AND website_id = $2",
        )
        .bind(listing_uuid)
        .bind(website_uuid)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Find sync record by content hash on a specific website
    pub async fn find_by_content_hash(
        website_id: WebsiteId,
        content_hash: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let website_uuid = website_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM listing_website_sync
             WHERE website_id = $1 AND content_hash = $2
             AND disappeared_at IS NULL",
        )
        .bind(website_uuid)
        .bind(content_hash)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all sync records for a website
    pub async fn find_all_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let website_uuid = website_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM listing_website_sync
             WHERE website_id = $1
             ORDER BY last_seen_at DESC",
        )
        .bind(website_uuid)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find active sync records for a website (not disappeared)
    pub async fn find_active_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let website_uuid = website_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM listing_website_sync
             WHERE website_id = $1 AND disappeared_at IS NULL
             ORDER BY last_seen_at DESC",
        )
        .bind(website_uuid)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all sync records for a listing
    pub async fn find_all_by_listing(listing_id: ListingId, pool: &PgPool) -> Result<Vec<Self>> {
        let listing_uuid = listing_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM listing_website_sync
             WHERE listing_id = $1
             ORDER BY last_seen_at DESC",
        )
        .bind(listing_uuid)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Upsert a sync record (create or update last_seen_at)
    ///
    /// This is the primary method for tracking listing appearances on websites.
    /// If the record already exists, it updates last_seen_at and clears disappeared_at.
    pub async fn upsert(
        listing_id: ListingId,
        website_id: WebsiteId,
        content_hash: String,
        source_url: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let listing_uuid = listing_id.into_uuid();
        let website_uuid = website_id.into_uuid();

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO listing_website_sync (
                listing_id,
                website_id,
                content_hash,
                source_url,
                first_seen_at,
                last_seen_at
            )
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (listing_id, website_id) DO UPDATE
            SET
                content_hash = EXCLUDED.content_hash,
                source_url = EXCLUDED.source_url,
                last_seen_at = NOW(),
                disappeared_at = NULL,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(listing_uuid)
        .bind(website_uuid)
        .bind(content_hash)
        .bind(source_url)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Mark listings as disappeared except for the given content hashes
    ///
    /// This is used after scraping a website to mark any listings that are no longer
    /// present. Returns the listing IDs that were marked as disappeared.
    pub async fn mark_disappeared_except(
        website_id: WebsiteId,
        content_hashes: Vec<String>,
        pool: &PgPool,
    ) -> Result<Vec<Uuid>> {
        let website_uuid = website_id.into_uuid();

        let listing_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE listing_website_sync
            SET disappeared_at = NOW(), updated_at = NOW()
            WHERE website_id = $1
              AND disappeared_at IS NULL
              AND content_hash != ALL($2)
            RETURNING listing_id
            "#,
        )
        .bind(website_uuid)
        .bind(&content_hashes)
        .fetch_all(pool)
        .await?;

        Ok(listing_ids)
    }

    /// Mark a specific sync record as disappeared
    pub async fn mark_disappeared(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE listing_website_sync
            SET disappeared_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get count of active listings for a website
    pub async fn count_active_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<i64> {
        let website_uuid = website_id.into_uuid();

        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM listing_website_sync
             WHERE website_id = $1 AND disappeared_at IS NULL",
        )
        .bind(website_uuid)
        .fetch_one(pool)
        .await?;

        Ok(count)
    }

    /// Get count of disappeared listings for a website
    pub async fn count_disappeared_by_website(
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<i64> {
        let website_uuid = website_id.into_uuid();

        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM listing_website_sync
             WHERE website_id = $1 AND disappeared_at IS NOT NULL",
        )
        .bind(website_uuid)
        .fetch_one(pool)
        .await?;

        Ok(count)
    }
}
