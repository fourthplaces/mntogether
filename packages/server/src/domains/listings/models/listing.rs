use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ContainerId, ListingId, OrganizationId, DomainId};

/// Listing - a service, opportunity, or business listing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Listing {
    pub id: ListingId,
    pub organization_id: Option<OrganizationId>,
    pub organization_name: String,

    // Content
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,

    // Hot path fields (hybrid approach)
    pub listing_type: String, // 'service', 'opportunity', 'business'
    pub category: String,
    pub capacity_status: Option<String>, // 'accepting', 'paused', 'at_capacity'
    pub urgency: Option<String>,         // 'low', 'medium', 'high', 'urgent'
    pub status: String, // 'pending_approval', 'active', 'filled', 'rejected', 'expired'

    // Verification
    pub verified_at: Option<DateTime<Utc>>,

    // Language
    pub source_language: String,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Submission tracking
    pub submission_type: Option<String>, // 'scraped', 'admin', 'org_submitted'
    pub submitted_by_admin_id: Option<Uuid>,

    // Sync tracking (for scraped listings)
    pub domain_id: Option<DomainId>,
    pub source_url: Option<String>, // Specific page URL where listing was found
    pub last_seen_at: DateTime<Utc>,
    pub disappeared_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,

    // Vector search (for semantic matching)
    pub embedding: Option<pgvector::Vector>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// Enums for type-safe edges
// =============================================================================

/// Listing type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListingType {
    Service,
    Opportunity,
    Business,
}

impl std::fmt::Display for ListingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingType::Service => write!(f, "service"),
            ListingType::Opportunity => write!(f, "opportunity"),
            ListingType::Business => write!(f, "business"),
        }
    }
}

impl std::str::FromStr for ListingType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "service" => Ok(ListingType::Service),
            "opportunity" => Ok(ListingType::Opportunity),
            "business" => Ok(ListingType::Business),
            _ => Err(anyhow::anyhow!("Invalid listing type: {}", s)),
        }
    }
}

/// Capacity status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapacityStatus {
    Accepting,
    Paused,
    AtCapacity,
}

impl std::fmt::Display for CapacityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapacityStatus::Accepting => write!(f, "accepting"),
            CapacityStatus::Paused => write!(f, "paused"),
            CapacityStatus::AtCapacity => write!(f, "at_capacity"),
        }
    }
}

impl std::str::FromStr for CapacityStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "accepting" => Ok(CapacityStatus::Accepting),
            "paused" => Ok(CapacityStatus::Paused),
            "at_capacity" => Ok(CapacityStatus::AtCapacity),
            _ => Err(anyhow::anyhow!("Invalid capacity status: {}", s)),
        }
    }
}

/// Urgency enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Medium,
    High,
    Urgent,
}

impl std::fmt::Display for Urgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Urgency::Low => write!(f, "low"),
            Urgency::Medium => write!(f, "medium"),
            Urgency::High => write!(f, "high"),
            Urgency::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for Urgency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "low" => Ok(Urgency::Low),
            "medium" => Ok(Urgency::Medium),
            "high" => Ok(Urgency::High),
            "urgent" => Ok(Urgency::Urgent),
            _ => Err(anyhow::anyhow!("Invalid urgency: {}", s)),
        }
    }
}

/// Status enum for type-safe edges
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    PendingApproval,
    Active,
    Filled,
    Rejected,
    Expired,
}

impl std::fmt::Display for ListingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingStatus::PendingApproval => write!(f, "pending_approval"),
            ListingStatus::Active => write!(f, "active"),
            ListingStatus::Filled => write!(f, "filled"),
            ListingStatus::Rejected => write!(f, "rejected"),
            ListingStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for ListingStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_approval" => Ok(ListingStatus::PendingApproval),
            "active" => Ok(ListingStatus::Active),
            "filled" => Ok(ListingStatus::Filled),
            "rejected" => Ok(ListingStatus::Rejected),
            "expired" => Ok(ListingStatus::Expired),
            _ => Err(anyhow::anyhow!("Invalid listing status: {}", s)),
        }
    }
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Listing {
    /// Find listing by ID
    pub async fn find_by_id(id: ListingId, pool: &PgPool) -> Result<Self> {
        let listing = sqlx::query_as::<_, Listing>("SELECT * FROM listings WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(listing)
    }

    /// Find listings by status
    pub async fn find_by_status(
        status: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Listing>(
            "SELECT * FROM listings
             WHERE status = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listings by listing type
    pub async fn find_by_type(
        listing_type: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Listing>(
            "SELECT * FROM listings
             WHERE listing_type = $1 AND status = 'active'
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(listing_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listings by category
    pub async fn find_by_category(
        category: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Listing>(
            "SELECT * FROM listings
             WHERE category = $1 AND status = 'active'
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listings by capacity status
    pub async fn find_by_capacity(
        capacity_status: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Listing>(
            "SELECT * FROM listings
             WHERE capacity_status = $1 AND status = 'active'
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(capacity_status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listings by domain ID
    pub async fn find_by_domain_id(domain_id: DomainId, pool: &PgPool) -> Result<Vec<Self>> {
        let listings =
            sqlx::query_as::<_, Listing>("SELECT * FROM listings WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_all(pool)
                .await?;
        Ok(listings)
    }

    /// Find listing by content hash
    pub async fn find_by_content_hash(content_hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        let listing = sqlx::query_as::<_, Listing>(
            "SELECT * FROM listings WHERE content_hash = $1 LIMIT 1",
        )
        .bind(content_hash)
        .fetch_optional(pool)
        .await?;
        Ok(listing)
    }

    /// Create a new listing (returns inserted record with defaults applied)
    pub async fn create(
        organization_name: String,
        title: String,
        description: String,
        tldr: Option<String>,
        listing_type: String,
        category: String,
        capacity_status: Option<String>,
        urgency: Option<String>,
        location: Option<String>,
        status: String,
        content_hash: Option<String>,
        source_language: String,
        submission_type: Option<String>,
        submitted_by_admin_id: Option<Uuid>,
        domain_id: Option<DomainId>,
        source_url: Option<String>,
        organization_id: Option<OrganizationId>,
        pool: &PgPool,
    ) -> Result<Self> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            INSERT INTO listings (
                organization_name,
                title,
                description,
                tldr,
                listing_type,
                category,
                capacity_status,
                urgency,
                location,
                status,
                content_hash,
                source_language,
                submission_type,
                submitted_by_admin_id,
                domain_id,
                source_url,
                organization_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#,
        )
        .bind(organization_name)
        .bind(title)
        .bind(description)
        .bind(tldr)
        .bind(listing_type)
        .bind(category)
        .bind(capacity_status)
        .bind(urgency)
        .bind(location)
        .bind(status)
        .bind(content_hash)
        .bind(source_language)
        .bind(submission_type)
        .bind(submitted_by_admin_id)
        .bind(domain_id)
        .bind(source_url)
        .bind(organization_id)
        .fetch_one(pool)
        .await?;

        Ok(listing)
    }

    /// Update listing status
    pub async fn update_status(id: ListingId, status: &str, pool: &PgPool) -> Result<Self> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(status)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(listing)
    }

    /// Update capacity status
    pub async fn update_capacity_status(
        id: ListingId,
        capacity_status: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings
            SET capacity_status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(capacity_status)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(listing)
    }

    /// Update listing content (for edit + approve)
    pub async fn update_content(
        id: ListingId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        category: Option<String>,
        urgency: Option<String>,
        location: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                description_markdown = COALESCE($4, description_markdown),
                tldr = COALESCE($5, tldr),
                category = COALESCE($6, category),
                urgency = COALESCE($7, urgency),
                location = COALESCE($8, location),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(description_markdown)
        .bind(tldr)
        .bind(category)
        .bind(urgency)
        .bind(location)
        .fetch_one(pool)
        .await?;
        Ok(listing)
    }

    /// Mark listings as disappeared (for sync)
    pub async fn mark_disappeared(listing_ids: &[ListingId], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE listings
            SET disappeared_at = NOW(), status = 'expired', updated_at = NOW()
            WHERE id = ANY($1) AND disappeared_at IS NULL
            "#,
        )
        .bind(listing_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update last_seen_at timestamp
    pub async fn update_last_seen(listing_ids: &[ListingId], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE listings
            SET last_seen_at = NOW(), updated_at = NOW()
            WHERE id = ANY($1)
            "#,
        )
        .bind(listing_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update listing embedding
    pub async fn update_embedding(id: ListingId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE listings SET embedding = $2 WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Find existing active listings from a domain (for sync)
    pub async fn find_active_by_domain(domain_id: DomainId, pool: &PgPool) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Listing>(
            r#"
            SELECT *
            FROM listings
            WHERE domain_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
            "#,
        )
        .bind(domain_id)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listing by domain and title (for sync - detecting changed listings)
    pub async fn find_by_domain_and_title(
        domain_id: DomainId,
        title: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            SELECT *
            FROM listings
            WHERE domain_id = $1
              AND title = $2
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(domain_id)
        .bind(title)
        .fetch_optional(pool)
        .await?;
        Ok(listing)
    }

    /// Mark listings as disappeared that are not in the provided content hash list (for sync)
    pub async fn mark_disappeared_except(
        domain_id: DomainId,
        content_hashes: &[String],
        pool: &PgPool,
    ) -> Result<Vec<ListingId>> {
        let disappeared_ids = sqlx::query_scalar::<_, ListingId>(
            r#"
            UPDATE listings
            SET disappeared_at = NOW(), updated_at = NOW()
            WHERE domain_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
              AND content_hash NOT IN (SELECT * FROM UNNEST($2::text[]))
            RETURNING id
            "#,
        )
        .bind(domain_id)
        .bind(content_hashes)
        .fetch_all(pool)
        .await?;
        Ok(disappeared_ids)
    }

    /// Update last_seen_at for a specific listing
    pub async fn touch_last_seen(id: ListingId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE listings
            SET last_seen_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Count listings by status (for pagination)
    pub async fn count_by_status(status: &str, pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM listings
            WHERE status = $1
            "#,
        )
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Find listing by content hash with status filter (for duplicate detection)
    pub async fn find_id_by_content_hash_active(
        content_hash: &str,
        pool: &PgPool,
    ) -> Result<Option<ListingId>> {
        let id = sqlx::query_scalar::<_, ListingId>(
            r#"
            SELECT id
            FROM listings
            WHERE content_hash = $1
              AND status IN ('pending_approval', 'active')
            LIMIT 1
            "#,
        )
        .bind(content_hash)
        .fetch_optional(pool)
        .await?;
        Ok(id)
    }

    /// Ensure listing is active (for operations that require active status)
    pub fn ensure_active(&self) -> Result<()> {
        if self.status != "active" {
            anyhow::bail!(
                "Listing must be active to perform this operation (current status: {})",
                self.status
            );
        }
        Ok(())
    }

    /// Delete a listing by ID
    pub async fn delete(id: ListingId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM listings WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Mark listing as verified
    pub async fn mark_verified(id: ListingId, pool: &PgPool) -> Result<Self> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings
            SET verified_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(listing)
    }

    /// Get or create a comments container for this listing
    pub async fn get_or_create_comments_container(&self, pool: &PgPool) -> Result<ContainerId> {
        // Check if container already exists
        let existing: Option<uuid::Uuid> = sqlx::query_scalar(
            "SELECT id FROM containers WHERE container_type = 'listing_comments' AND entity_id = $1",
        )
        .bind(self.id.as_uuid())
        .fetch_optional(pool)
        .await?;

        if let Some(container_id) = existing {
            return Ok(ContainerId::from(container_id));
        }

        // Create new container
        let container_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO containers (container_type, entity_id, language)
            VALUES ('listing_comments', $1, $2)
            RETURNING id
            "#,
        )
        .bind(self.id.as_uuid())
        .bind(&self.source_language)
        .fetch_one(pool)
        .await?;

        Ok(ContainerId::from(container_id))
    }

    /// Get comments container ID if it exists
    pub async fn get_comments_container_id(&self, pool: &PgPool) -> Result<Option<ContainerId>> {
        let container_id: Option<uuid::Uuid> = sqlx::query_scalar(
            "SELECT id FROM containers WHERE container_type = 'listing_comments' AND entity_id = $1",
        )
        .bind(self.id.as_uuid())
        .fetch_optional(pool)
        .await?;

        Ok(container_id.map(ContainerId::from))
    }
}
