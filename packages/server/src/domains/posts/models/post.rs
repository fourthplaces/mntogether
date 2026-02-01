use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ContainerId, PostId, OrganizationId, WebsiteId};

/// Listing - a service, opportunity, or business listing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: PostId,
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

    // Source tracking (for scraped listings)
    pub website_id: Option<WebsiteId>,
    pub source_url: Option<String>, // Specific page URL where listing was found (for traceability)

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
pub enum PostType {
    Service,      // Food shelf, legal aid, housing help
    Professional, // Lawyer, doctor, social worker
    Business,     // Restaurant, shop
    Opportunity,  // Volunteer role, job, event
}

impl std::fmt::Display for PostType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostType::Service => write!(f, "service"),
            PostType::Professional => write!(f, "professional"),
            PostType::Business => write!(f, "business"),
            PostType::Opportunity => write!(f, "opportunity"),
        }
    }
}

impl std::str::FromStr for PostType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "service" => Ok(PostType::Service),
            "professional" => Ok(PostType::Professional),
            "business" => Ok(PostType::Business),
            "opportunity" => Ok(PostType::Opportunity),
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
pub enum PostStatus {
    PendingApproval,
    Active,
    Filled,
    Rejected,
    Expired,
}

impl std::fmt::Display for PostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostStatus::PendingApproval => write!(f, "pending_approval"),
            PostStatus::Active => write!(f, "active"),
            PostStatus::Filled => write!(f, "filled"),
            PostStatus::Rejected => write!(f, "rejected"),
            PostStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for PostStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_approval" => Ok(PostStatus::PendingApproval),
            "active" => Ok(PostStatus::Active),
            "filled" => Ok(PostStatus::Filled),
            "rejected" => Ok(PostStatus::Rejected),
            "expired" => Ok(PostStatus::Expired),
            _ => Err(anyhow::anyhow!("Invalid listing status: {}", s)),
        }
    }
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Post {
    /// Find listing by ID
    pub async fn find_by_id(id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(post)
    }

    /// Find listings by status
    pub async fn find_by_status(
        status: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts
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
        let listings = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts
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
        let listings = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts
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
        let listings = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts
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
    pub async fn find_by_website_id(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE website_id = $1")
            .bind(website_id)
            .fetch_all(pool)
            .await?;
        Ok(listings)
    }

    /// Find listing by content hash
    pub async fn find_by_content_hash(content_hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        let post =
            sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE content_hash = $1 LIMIT 1")
                .bind(content_hash)
                .fetch_optional(pool)
                .await?;
        Ok(post)
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
        source_language: String,
        submission_type: Option<String>,
        submitted_by_admin_id: Option<Uuid>,
        website_id: Option<WebsiteId>,
        source_url: Option<String>,
        organization_id: Option<OrganizationId>,
        pool: &PgPool,
    ) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (
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
                source_language,
                submission_type,
                submitted_by_admin_id,
                website_id,
                source_url,
                organization_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
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
        .bind(source_language)
        .bind(submission_type)
        .bind(submitted_by_admin_id)
        .bind(website_id)
        .bind(source_url)
        .bind(organization_id)
        .fetch_one(pool)
        .await?;

        Ok(post)
    }

    /// Update listing status
    pub async fn update_status(id: PostId, status: &str, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(status)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }

    /// Update capacity status
    pub async fn update_capacity_status(
        id: PostId,
        capacity_status: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET capacity_status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(capacity_status)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }

    /// Update listing content (for edit + approve)
    pub async fn update_content(
        id: PostId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        category: Option<String>,
        urgency: Option<String>,
        location: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
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
        Ok(post)
    }

    /// Mark listings as disappeared (for sync)
    pub async fn mark_disappeared(post_ids: &[PostId], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE posts
            SET disappeared_at = NOW(), status = 'expired', updated_at = NOW()
            WHERE id = ANY($1) AND disappeared_at IS NULL
            "#,
        )
        .bind(post_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update last_seen_at timestamp
    pub async fn update_last_seen(post_ids: &[PostId], pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE posts
            SET last_seen_at = NOW(), updated_at = NOW()
            WHERE id = ANY($1)
            "#,
        )
        .bind(post_ids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update listing embedding
    pub async fn update_embedding(id: PostId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE posts SET embedding = $2 WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Find existing active listings from a domain (for sync)
    pub async fn find_active_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Post>(
            r#"
            SELECT *
            FROM posts
            WHERE website_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
            "#,
        )
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listing by domain and title (for sync - detecting changed listings)
    pub async fn find_by_domain_and_title(
        website_id: WebsiteId,
        title: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            SELECT *
            FROM posts
            WHERE website_id = $1
              AND title = $2
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(website_id)
        .bind(title)
        .fetch_optional(pool)
        .await?;
        Ok(post)
    }

    /// Mark listings as disappeared that are not in the provided content hash list (for sync)
    pub async fn mark_disappeared_except(
        website_id: WebsiteId,
        content_hashes: &[String],
        pool: &PgPool,
    ) -> Result<Vec<PostId>> {
        let disappeared_ids = sqlx::query_scalar::<_, PostId>(
            r#"
            UPDATE posts
            SET disappeared_at = NOW(), updated_at = NOW()
            WHERE website_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
              AND content_hash NOT IN (SELECT * FROM UNNEST($2::text[]))
            RETURNING id
            "#,
        )
        .bind(website_id)
        .bind(content_hashes)
        .fetch_all(pool)
        .await?;
        Ok(disappeared_ids)
    }

    /// Update last_seen_at for a specific listing
    pub async fn touch_last_seen(id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE posts
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
            FROM posts
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
    ) -> Result<Option<PostId>> {
        let id = sqlx::query_scalar::<_, PostId>(
            r#"
            SELECT id
            FROM posts
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
    pub async fn delete(id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Mark listing as verified
    pub async fn mark_verified(id: PostId, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET verified_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(post)
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

    /// Find listings without embeddings (for batch embedding generation)
    pub async fn find_without_embeddings(limit: i64, pool: &PgPool) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM posts
            WHERE embedding IS NULL
              AND status IN ('pending_approval', 'active')
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find listings without embeddings for a specific website
    pub async fn find_without_embeddings_for_website(
        website_id: WebsiteId,
        limit: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM posts
            WHERE embedding IS NULL
              AND website_id = $1
              AND status IN ('pending_approval', 'active')
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(website_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Count listings without embeddings
    pub async fn count_without_embeddings(pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM posts WHERE embedding IS NULL AND status IN ('pending_approval', 'active')",
        )
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Count listings without embeddings for a specific website
    pub async fn count_without_embeddings_for_website(
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM posts WHERE embedding IS NULL AND website_id = $1 AND status IN ('pending_approval', 'active')",
        )
        .bind(website_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}
