use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::common::{
    ContainerId, PaginationDirection, PostId, SocialProfileId, ValidatedPaginationArgs, WebsiteId,
};
use crate::domains::schedules::models::Schedule;

/// Listing - a service, opportunity, or business listing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: PostId,

    // Content
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub summary: Option<String>,

    // Hot path fields (hybrid approach)
    pub post_type: String, // 'service', 'opportunity', 'business'
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

    // Who submitted this post (member FK — both humans and agents are members)
    pub submitted_by_id: Option<Uuid>,

    // Source tracking (for scraped listings)
    pub website_id: Option<WebsiteId>,
    pub social_profile_id: Option<SocialProfileId>,
    pub source_url: Option<String>, // Specific page URL where listing was found (for traceability)

    // Soft delete (preserves links)
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_reason: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Vector search (for semantic search)
    pub embedding: Option<pgvector::Vector>,

    // Revision tracking (for draft mode)
    pub revision_of_post_id: Option<PostId>,

    // Translation tracking
    pub translation_of_id: Option<PostId>,

    // Comments container (inverted FK from containers table)
    pub comments_container_id: Option<ContainerId>,
}

/// Search result from semantic similarity search
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostSearchResult {
    pub post_id: PostId,
    pub title: String,
    pub description: String,
    pub category: String,
    pub post_type: String,
    pub similarity: f64,
}

/// Post with distance info for proximity search
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostWithDistance {
    pub id: PostId,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub summary: Option<String>,
    pub post_type: String,
    pub category: String,
    pub status: String,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub source_url: Option<String>,
    pub website_id: Option<WebsiteId>,
    pub created_at: DateTime<Utc>,
    pub zip_code: Option<String>,
    pub location_city: Option<String>,
    pub distance_miles: f64,
}

/// Search result with location info (for chat agent tool)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostSearchResultWithLocation {
    pub post_id: PostId,
    pub title: String,
    pub description: String,
    pub summary: Option<String>,
    pub category: String,
    pub post_type: String,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub similarity: f64,
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
// Builder Structs
// =============================================================================

/// Builder for creating a new Post
#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct CreatePost {
    // Required fields - no default
    pub title: String,
    pub description: String,

    // Optional fields - have defaults
    #[builder(default)]
    pub summary: Option<String>,
    #[builder(default = "opportunity".to_string())]
    pub post_type: String,
    #[builder(default = "general".to_string())]
    pub category: String,
    #[builder(default)]
    pub capacity_status: Option<String>,
    #[builder(default)]
    pub urgency: Option<String>,
    #[builder(default)]
    pub location: Option<String>,
    #[builder(default = "pending_approval".to_string())]
    pub status: String,
    #[builder(default = "en".to_string())]
    pub source_language: String,
    #[builder(default)]
    pub submission_type: Option<String>,
    #[builder(default)]
    pub submitted_by_id: Option<Uuid>,
    #[builder(default)]
    pub website_id: Option<WebsiteId>,
    #[builder(default)]
    pub source_url: Option<String>,
    #[builder(default)]
    pub revision_of_post_id: Option<PostId>,
    #[builder(default)]
    pub translation_of_id: Option<PostId>,
}

/// Builder for updating Post content
#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct UpdatePostContent {
    pub id: PostId,
    #[builder(default)]
    pub title: Option<String>,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub description_markdown: Option<String>,
    #[builder(default)]
    pub summary: Option<String>,
    #[builder(default)]
    pub category: Option<String>,
    #[builder(default)]
    pub urgency: Option<String>,
    #[builder(default)]
    pub location: Option<String>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Post {
    /// Find all posts created by a specific agent (joins through agents.member_id).
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT p.* FROM posts p
            JOIN agents a ON a.member_id = p.submitted_by_id
            WHERE a.id = $1 AND p.deleted_at IS NULL
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Count posts created by a specific agent (joins through agents.member_id).
    pub async fn count_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM posts p
            JOIN agents a ON a.member_id = p.submitted_by_id
            WHERE a.id = $1 AND p.deleted_at IS NULL
            "#,
        )
        .bind(agent_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Count posts by agent grouped by website_id (for per-website counts).
    pub async fn count_by_agent_grouped_by_website(
        agent_id: Uuid,
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<Uuid, i64>> {
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT p.website_id, COUNT(*) as count
            FROM posts p
            JOIN agents a ON a.member_id = p.submitted_by_id
            WHERE a.id = $1
              AND p.website_id IS NOT NULL
              AND p.deleted_at IS NULL
            GROUP BY p.website_id
            "#,
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Batch-load posts by IDs (for DataLoader)
    pub async fn find_by_ids(ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM posts WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Batch-load post titles by IDs (includes soft-deleted posts, for display purposes)
    pub async fn find_titles_by_ids(
        ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<Vec<(Uuid, String)>> {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, title FROM posts WHERE id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

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
             WHERE status = $1 AND deleted_at IS NULL AND revision_of_post_id IS NULL AND translation_of_id IS NULL
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

    /// Find listings with cursor-based pagination (Relay spec)
    ///
    /// Uses V7 UUID ordering (time-based) for stable pagination.
    /// Fetches limit+1 to detect if there are more pages.
    /// When agent_id is provided, filters via JOIN through agents.member_id.
    pub async fn find_paginated(
        status: Option<&str>,
        website_id: Option<WebsiteId>,
        agent_id: Option<uuid::Uuid>,
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    r#"
                    SELECT p.* FROM posts p
                    LEFT JOIN agents a ON a.member_id = p.submitted_by_id
                    WHERE ($1::text IS NULL OR p.status = $1)
                      AND p.deleted_at IS NULL
                      AND p.revision_of_post_id IS NULL
                      AND p.translation_of_id IS NULL
                      AND ($2::uuid IS NULL OR p.id > $2)
                      AND ($4::uuid IS NULL OR p.website_id = $4)
                      AND ($5::uuid IS NULL OR a.id = $5)
                    ORDER BY p.id ASC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(website_id)
                .bind(agent_id)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                // Fetch in reverse order, then re-sort
                let mut rows = sqlx::query_as::<_, Self>(
                    r#"
                    SELECT p.* FROM posts p
                    LEFT JOIN agents a ON a.member_id = p.submitted_by_id
                    WHERE ($1::text IS NULL OR p.status = $1)
                      AND p.deleted_at IS NULL
                      AND p.revision_of_post_id IS NULL
                      AND p.translation_of_id IS NULL
                      AND ($2::uuid IS NULL OR p.id < $2)
                      AND ($4::uuid IS NULL OR p.website_id = $4)
                      AND ($5::uuid IS NULL OR a.id = $5)
                    ORDER BY p.id DESC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(website_id)
                .bind(agent_id)
                .fetch_all(pool)
                .await?;

                // Re-sort to ascending order
                rows.reverse();
                rows
            }
        };

        // Check if there are more pages
        let has_more = results.len() > args.limit as usize;

        // Trim to requested limit
        let results = if has_more {
            results.into_iter().take(args.limit as usize).collect()
        } else {
            results
        };

        Ok((results, has_more))
    }

    /// Find listings by listing type
    pub async fn find_by_type(
        post_type: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts
             WHERE post_type = $1 AND status = 'active' AND deleted_at IS NULL AND revision_of_post_id IS NULL AND translation_of_id IS NULL
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(post_type)
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
             WHERE category = $1 AND status = 'active' AND deleted_at IS NULL AND revision_of_post_id IS NULL AND translation_of_id IS NULL
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
             WHERE capacity_status = $1 AND status = 'active' AND deleted_at IS NULL AND revision_of_post_id IS NULL AND translation_of_id IS NULL
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

    /// Find listings by domain ID (excludes soft-deleted and revisions)
    pub async fn find_by_website_id(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts WHERE website_id = $1 AND deleted_at IS NULL AND revision_of_post_id IS NULL AND translation_of_id IS NULL",
        )
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(listings)
    }

    /// Find posts for a specific agent on a specific website.
    /// Joins through agents to match by agent_id → member_id → submitted_by_id.
    pub async fn find_by_agent_and_website(
        agent_id: uuid::Uuid,
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT p.*
            FROM posts p
            JOIN agents a ON a.member_id = p.submitted_by_id
            WHERE a.id = $1
              AND p.website_id = $2
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(agent_id)
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(posts)
    }

    /// Count posts grouped by website_id for a set of website IDs
    pub async fn count_by_website_ids(
        website_ids: &[uuid::Uuid],
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<uuid::Uuid, i64>> {
        let rows = sqlx::query_as::<_, (uuid::Uuid, i64)>(
            r#"
            SELECT website_id, COUNT(*) as count
            FROM posts
            WHERE website_id = ANY($1)
              AND deleted_at IS NULL
              AND revision_of_post_id IS NULL
              AND translation_of_id IS NULL
            GROUP BY website_id
            "#,
        )
        .bind(website_ids)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Create a new listing (returns inserted record with defaults applied)
    pub async fn create(input: CreatePost, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (
                title,
                description,
                summary,
                post_type,
                category,
                capacity_status,
                urgency,
                location,
                status,
                source_language,
                submission_type,
                submitted_by_id,
                website_id,
                source_url,
                revision_of_post_id,
                translation_of_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(input.title)
        .bind(input.description)
        .bind(input.summary)
        .bind(input.post_type)
        .bind(input.category)
        .bind(input.capacity_status)
        .bind(input.urgency)
        .bind(input.location)
        .bind(input.status)
        .bind(input.source_language)
        .bind(input.submission_type)
        .bind(input.submitted_by_id)
        .bind(input.website_id)
        .bind(input.source_url)
        .bind(input.revision_of_post_id)
        .bind(input.translation_of_id)
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
    pub async fn update_content(input: UpdatePostContent, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                description_markdown = COALESCE($4, description_markdown),
                summary = COALESCE($5, summary),
                category = COALESCE($6, category),
                urgency = COALESCE($7, urgency),
                location = COALESCE($8, location),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(input.id)
        .bind(input.title)
        .bind(input.description)
        .bind(input.description_markdown)
        .bind(input.summary)
        .bind(input.category)
        .bind(input.urgency)
        .bind(input.location)
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

    /// Find existing active listings from a domain (for sync)
    pub async fn find_active_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let listings = sqlx::query_as::<_, Post>(
            r#"
            SELECT *
            FROM posts
            WHERE website_id = $1
              AND status IN ('pending_approval', 'active')
              AND disappeared_at IS NULL
              AND deleted_at IS NULL
              AND revision_of_post_id IS NULL
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
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(website_id)
        .bind(title)
        .fetch_optional(pool)
        .await?;
        Ok(post)
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
    /// When agent_id is provided, filters via JOIN through agents.member_id.
    pub async fn count_by_status(
        status: Option<&str>,
        website_id: Option<WebsiteId>,
        agent_id: Option<uuid::Uuid>,
        pool: &PgPool,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM posts p
            LEFT JOIN agents a ON a.member_id = p.submitted_by_id
            WHERE ($1::text IS NULL OR p.status = $1)
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($2::uuid IS NULL OR p.website_id = $2)
              AND ($3::uuid IS NULL OR a.id = $3)
            "#,
        )
        .bind(status)
        .bind(website_id)
        .bind(agent_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
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

    /// Delete a listing by ID (hard delete - use soft_delete instead for link preservation)
    pub async fn delete(id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Soft delete a listing (preserves the record for link continuity)
    /// reason should explain why, e.g. "Duplicate of post <uuid>"
    pub async fn soft_delete(id: PostId, reason: &str, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE posts
            SET deleted_at = NOW(),
                deleted_reason = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(reason)
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

    /// Get or create a comments container for this post
    pub async fn get_or_create_comments_container(&self, pool: &PgPool) -> Result<ContainerId> {
        // Return existing container if set
        if let Some(container_id) = self.comments_container_id {
            return Ok(container_id);
        }

        // Create new container and set the FK on this post
        let container_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO containers (language)
            VALUES ($1)
            RETURNING id
            "#,
        )
        .bind(&self.source_language)
        .fetch_one(pool)
        .await?;

        // Update the post with the new container ID
        sqlx::query("UPDATE posts SET comments_container_id = $1 WHERE id = $2")
            .bind(container_id)
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(ContainerId::from(container_id))
    }

    /// Get comments container ID if it exists
    pub fn get_comments_container_id(&self) -> Option<ContainerId> {
        self.comments_container_id
    }

    // =========================================================================
    // Embedding Methods (for semantic search)
    // =========================================================================

    /// Update embedding for a post
    pub async fn update_embedding(id: PostId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE posts SET embedding = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Search posts by semantic similarity
    pub async fn search_by_similarity(
        query_embedding: &[f32],
        threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<PostSearchResult>> {
        use pgvector::Vector;

        let vector = Vector::from(query_embedding.to_vec());

        let results = sqlx::query_as::<_, PostSearchResult>(
            r#"
            SELECT
                p.id as post_id,
                p.title,
                p.description,
                p.category,
                p.post_type,
                (1 - (p.embedding <=> $1))::float8 as similarity
            FROM posts p
            WHERE p.embedding IS NOT NULL
              AND p.deleted_at IS NULL
              AND p.status = 'active'
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND (1 - (p.embedding <=> $1)) > $2
            ORDER BY p.embedding <=> $1
            LIMIT $3
            "#,
        )
        .bind(&vector)
        .bind(threshold)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(results)
    }

    /// Search posts by semantic similarity (with location in results)
    pub async fn search_by_similarity_with_location(
        query_embedding: &[f32],
        threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<PostSearchResultWithLocation>> {
        use pgvector::Vector;

        let vector = Vector::from(query_embedding.to_vec());

        let results = sqlx::query_as::<_, PostSearchResultWithLocation>(
            r#"
            SELECT
                p.id as post_id,
                p.title,
                p.description,
                p.summary,
                p.category,
                p.post_type,
                p.location,
                p.source_url,
                (1 - (p.embedding <=> $1))::float8 as similarity
            FROM posts p
            WHERE p.embedding IS NOT NULL
              AND p.deleted_at IS NULL
              AND p.status = 'active'
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND (1 - (p.embedding <=> $1)) > $2
            ORDER BY p.embedding <=> $1
            LIMIT $3
            "#,
        )
        .bind(&vector)
        .bind(threshold)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(results)
    }

    /// Find posts without embeddings (for backfill)
    pub async fn find_without_embeddings(limit: i32, pool: &PgPool) -> Result<Vec<Self>> {
        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE embedding IS NULL
              AND deleted_at IS NULL
              AND status = 'active'
              AND revision_of_post_id IS NULL
              AND translation_of_id IS NULL
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }

    /// Get text for embedding generation
    /// Combines title, description, summary, category, post_type, and location
    pub fn get_embedding_text(&self) -> String {
        let mut parts = vec![self.title.clone()];

        parts.push(self.description.clone());

        if let Some(ref summary) = self.summary {
            parts.push(summary.clone());
        }

        parts.push(format!("Category: {}", self.category));
        parts.push(format!("Type: {}", self.post_type));

        if let Some(ref location) = self.location {
            parts.push(format!("Location: {}", location));
        }

        parts.join(" | ")
    }

    // =========================================================================
    // Revision Methods (for draft review workflow)
    // =========================================================================

    /// Find all pending revisions (posts that are revisions of other posts)
    pub async fn find_pending_revisions(pool: &PgPool) -> Result<Vec<Self>> {
        let revisions = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE revision_of_post_id IS NOT NULL
              AND deleted_at IS NULL
              AND status = 'pending_approval'
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(revisions)
    }

    /// Find active posts near a zip code, ordered by distance
    pub async fn find_near_zip(
        center_zip: &str,
        radius_miles: f64,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<PostWithDistance>> {
        sqlx::query_as::<_, PostWithDistance>(
            r#"
            WITH center AS (
                SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1
            )
            SELECT p.id, p.title, p.description,
                   p.description_markdown, p.summary,
                   p.post_type, p.category, p.status, p.urgency,
                   p.location, p.submission_type, p.source_url,
                   p.website_id, p.created_at,
                   l.postal_code as zip_code, l.city as location_city,
                   haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude) as distance_miles
            FROM posts p
            INNER JOIN post_locations pl ON pl.post_id = p.id
            INNER JOIN locations l ON l.id = pl.location_id
            INNER JOIN zip_codes z ON l.postal_code = z.zip_code
            CROSS JOIN center c
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude) <= $2
            ORDER BY distance_miles ASC
            LIMIT $3
            "#,
        )
        .bind(center_zip)
        .bind(radius_miles)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find revision for a specific post (if any)
    pub async fn find_revision_for_post(post_id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        let revision = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE revision_of_post_id = $1
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?;
        Ok(revision)
    }

    /// Find revisions by website (for bulk operations)
    pub async fn find_revisions_by_website(
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let revisions = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE revision_of_post_id IS NOT NULL
              AND website_id = $1
              AND deleted_at IS NULL
              AND status = 'pending_approval'
            ORDER BY created_at DESC
            "#,
        )
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(revisions)
    }

    /// Delete a revision and update the original post with the revision's content
    /// Returns the updated original post
    pub async fn apply_revision(revision_id: PostId, pool: &PgPool) -> Result<Self> {
        let revision = Self::find_by_id(revision_id, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Revision not found"))?;

        let original_id = revision
            .revision_of_post_id
            .ok_or_else(|| anyhow::anyhow!("Not a revision post"))?;

        // Copy revision fields to original
        let updated = Self::update_content(
            UpdatePostContent::builder()
                .id(original_id)
                .title(Some(revision.title))
                .description(Some(revision.description))
                .description_markdown(revision.description_markdown)
                .summary(revision.summary)
                .category(Some(revision.category))
                .urgency(revision.urgency)
                .location(revision.location)
                .build(),
            pool,
        )
        .await?;

        // Delete the revision
        Self::delete(revision_id, pool).await?;

        Ok(updated)
    }

    // =========================================================================
    // Event Schedule Queries (joins against tags)
    // =========================================================================

    /// Find schedules for posts that have the `post_type: event` tag.
    /// Used by the upcoming_events query.
    pub async fn find_event_schedules(pool: &PgPool) -> Result<Vec<Schedule>> {
        sqlx::query_as::<_, Schedule>(
            r#"
            SELECT s.* FROM schedules s
            INNER JOIN taggables tg ON tg.taggable_type = 'post' AND tg.taggable_id = s.schedulable_id
            INNER JOIN tags t ON t.id = tg.tag_id
            WHERE s.schedulable_type = 'post'
              AND t.kind = 'post_type' AND t.value = 'event'
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    // =========================================================================
    // Translation Methods
    // =========================================================================

    /// Find all translations of a given post
    pub async fn find_translations_for_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM posts WHERE translation_of_id = $1 AND deleted_at IS NULL",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find pending (non-deleted, non-revision) posts for a website
    pub async fn find_pending_by_website(
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM posts
            WHERE website_id = $1
              AND status = 'pending_approval'
              AND deleted_at IS NULL
              AND revision_of_post_id IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(website_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find only active (published) posts for a website, excluding pending
    pub async fn find_active_only_by_website(
        website_id: WebsiteId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM posts
            WHERE website_id = $1
              AND status = 'active'
              AND deleted_at IS NULL
              AND revision_of_post_id IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(website_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a revision post by copying content from a source post, pointing at original_id
    pub async fn create_revision_from(
        source: &Post,
        original_id: PostId,
        pool: &PgPool,
    ) -> Result<Self> {
        Post::create(
            CreatePost::builder()
                .title(source.title.clone())
                .description(source.description.clone())
                .summary(source.summary.clone())
                .post_type(source.post_type.clone())
                .category(source.category.clone())
                .urgency(source.urgency.clone())
                .location(source.location.clone())
                .source_language(source.source_language.clone())
                .submission_type(Some("revision".to_string()))
                .website_id(source.website_id)
                .source_url(source.source_url.clone())
                .revision_of_post_id(Some(original_id))
                .build(),
            pool,
        )
        .await
    }

    // =========================================================================
    // Public Filtered Queries (for home page directory)
    // =========================================================================

    /// Find active posts with optional post_type and category tag filters.
    ///
    /// - `post_type`: a `post_type` tag value like "seeking", "offering", "announcement"
    /// - `category`: a `service_offered` tag value like "food-assistance", "legal-aid"
    pub async fn find_public_filtered(
        post_type: Option<&str>,
        category: Option<&str>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT DISTINCT p.* FROM posts p
            LEFT JOIN taggables tg_pt ON tg_pt.taggable_type = 'post' AND tg_pt.taggable_id = p.id
            LEFT JOIN tags t_pt ON t_pt.id = tg_pt.tag_id AND t_pt.kind = 'post_type'
            LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
            LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($1::text IS NULL OR t_pt.value = $1)
              AND ($2::text IS NULL OR t_cat.value = $2)
            ORDER BY p.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(post_type)
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Count active posts matching the same filters as find_public_filtered
    pub async fn count_public_filtered(
        post_type: Option<&str>,
        category: Option<&str>,
        pool: &PgPool,
    ) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(DISTINCT p.id) FROM posts p
            LEFT JOIN taggables tg_pt ON tg_pt.taggable_type = 'post' AND tg_pt.taggable_id = p.id
            LEFT JOIN tags t_pt ON t_pt.id = tg_pt.tag_id AND t_pt.kind = 'post_type'
            LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
            LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($1::text IS NULL OR t_pt.value = $1)
              AND ($2::text IS NULL OR t_cat.value = $2)
            "#,
        )
        .bind(post_type)
        .bind(category)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a translation of a post in a specific language
    pub async fn find_translation(
        post_id: PostId,
        language: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM posts WHERE translation_of_id = $1 AND source_language = $2 AND deleted_at IS NULL LIMIT 1",
        )
        .bind(post_id)
        .bind(language)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}
