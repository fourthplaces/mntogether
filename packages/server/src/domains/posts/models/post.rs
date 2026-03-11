use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::common::{ContainerId, PaginationDirection, PostId, ValidatedPaginationArgs};
use crate::domains::schedules::models::Schedule;

/// A post — community content in one of 6 types (story, notice, exchange, event, spotlight, reference).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: PostId,

    // Content
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub summary: Option<String>,

    // Weight-specific body text (populated by Root Signal)
    pub body_heavy: Option<String>,
    pub body_medium: Option<String>,
    pub body_light: Option<String>,

    // Type system (Phase 2)
    pub post_type: String, // 'story', 'notice', 'exchange', 'event', 'spotlight', 'reference'
    pub category: String,
    pub weight: String,  // 'heavy', 'medium', 'light' — layout column width
    pub priority: i32,   // editorial importance (higher = more prominent)
    pub urgency: Option<String>,         // 'none', 'notice', 'urgent'
    pub status: String, // 'draft', 'active', 'filled', 'rejected', 'expired', 'archived'

    // Verification
    pub verified_at: Option<DateTime<Utc>>,

    // Language
    pub source_language: String,

    // Location
    pub location: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub zip_code: Option<String>,

    // Submission tracking
    pub submission_type: Option<String>, // 'scraped', 'admin', 'org_submitted'

    // Who submitted this post (member FK)
    pub submitted_by_id: Option<Uuid>,

    // Source tracking (for scraped listings)
    pub source_url: Option<String>, // Specific page URL where listing was found (for traceability)

    // Soft delete (preserves links)
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_reason: Option<String>,

    // Original publication date (e.g. from Instagram timestamp)
    pub published_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Revision tracking (for draft mode)
    pub revision_of_post_id: Option<PostId>,

    // Translation tracking
    pub translation_of_id: Option<PostId>,

    // Deduplication tracking (points to the canonical post this was merged into)
    pub duplicate_of_id: Option<PostId>,

    // Comments container (inverted FK from containers table)
    pub comments_container_id: Option<ContainerId>,

    // Full-text search vector (auto-managed by DB trigger, never read in app code)
    #[sqlx(skip)]
    #[serde(skip)]
    pub search_vector: Option<String>,
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
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub zip_code: Option<String>,
    pub location_city: Option<String>,
    pub distance_miles: f64,
}

/// PostWithDistance plus total_count from COUNT(*) OVER() window function
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostWithDistanceAndCount {
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
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub distance_miles: f64,
    pub total_count: i64,
}

// =============================================================================
// Enums for type-safe edges
// =============================================================================

/// Post type enum — form presets, not rigid schemas.
/// Types set which field groups are open by default; all groups are available on all types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostType {
    Story,     // Feature articles, narratives
    Notice,    // Announcements, alerts, public notices
    Exchange,  // Needs/offers, services, opportunities
    Event,     // Calendar events with datetime/location
    Spotlight, // Community member or business profiles
    Reference, // Directories, resource lists
}

impl std::fmt::Display for PostType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostType::Story => write!(f, "story"),
            PostType::Notice => write!(f, "notice"),
            PostType::Exchange => write!(f, "exchange"),
            PostType::Event => write!(f, "event"),
            PostType::Spotlight => write!(f, "spotlight"),
            PostType::Reference => write!(f, "reference"),
        }
    }
}

impl std::str::FromStr for PostType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "story" => Ok(PostType::Story),
            "notice" => Ok(PostType::Notice),
            "exchange" => Ok(PostType::Exchange),
            "event" => Ok(PostType::Event),
            "spotlight" => Ok(PostType::Spotlight),
            "reference" => Ok(PostType::Reference),
            _ => Err(anyhow::anyhow!("Invalid post type: {}", s)),
        }
    }
}

/// Layout weight — determines column width on the broadsheet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Weight {
    Heavy,  // Full-width feature
    Medium, // Half-width
    Light,  // Third-width or sidebar
}

impl std::fmt::Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Weight::Heavy => write!(f, "heavy"),
            Weight::Medium => write!(f, "medium"),
            Weight::Light => write!(f, "light"),
        }
    }
}

impl std::str::FromStr for Weight {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "heavy" => Ok(Weight::Heavy),
            "medium" => Ok(Weight::Medium),
            "light" => Ok(Weight::Light),
            _ => Err(anyhow::anyhow!("Invalid weight: {}", s)),
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

/// Urgency enum — NULL means no urgency (info-level)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Notice,
    Urgent,
}

impl std::fmt::Display for Urgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Urgency::Notice => write!(f, "notice"),
            Urgency::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for Urgency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "notice" => Ok(Urgency::Notice),
            "urgent" => Ok(Urgency::Urgent),
            _ => Err(anyhow::anyhow!("Invalid urgency: {}", s)),
        }
    }
}

/// Status enum for type-safe edges
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    PendingApproval, // Legacy — kept for backward compat, not used for new posts
    Active,
    Filled,
    Rejected,
    Expired,
    Archived,
}

impl std::fmt::Display for PostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostStatus::Draft => write!(f, "draft"),
            PostStatus::PendingApproval => write!(f, "pending_approval"),
            PostStatus::Active => write!(f, "active"),
            PostStatus::Filled => write!(f, "filled"),
            PostStatus::Rejected => write!(f, "rejected"),
            PostStatus::Expired => write!(f, "expired"),
            PostStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for PostStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "draft" => Ok(PostStatus::Draft),
            "pending_approval" => Ok(PostStatus::PendingApproval),
            "active" => Ok(PostStatus::Active),
            "filled" => Ok(PostStatus::Filled),
            "rejected" => Ok(PostStatus::Rejected),
            "expired" => Ok(PostStatus::Expired),
            "archived" => Ok(PostStatus::Archived),
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
    #[builder(default = "notice".to_string())]
    pub post_type: String,
    #[builder(default = "general".to_string())]
    pub category: String,
    #[builder(default = "medium".to_string())]
    pub weight: String,
    #[builder(default)]
    pub priority: i32,
    #[builder(default)]
    pub urgency: Option<String>,
    #[builder(default)]
    pub location: Option<String>,
    #[builder(default = "active".to_string())]
    pub status: String,
    #[builder(default = "en".to_string())]
    pub source_language: String,
    #[builder(default)]
    pub submission_type: Option<String>,
    #[builder(default)]
    pub submitted_by_id: Option<Uuid>,
    #[builder(default)]
    pub description_markdown: Option<String>,
    #[builder(default)]
    pub source_url: Option<String>,
    #[builder(default)]
    pub revision_of_post_id: Option<PostId>,
    #[builder(default)]
    pub translation_of_id: Option<PostId>,
    #[builder(default)]
    pub published_at: Option<DateTime<Utc>>,
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
    pub post_type: Option<String>,
    #[builder(default)]
    pub category: Option<String>,
    #[builder(default)]
    pub weight: Option<String>,
    #[builder(default)]
    pub priority: Option<i32>,
    #[builder(default)]
    pub urgency: Option<String>,
    #[builder(default)]
    pub location: Option<String>,
    #[builder(default)]
    pub zip_code: Option<String>,
    #[builder(default)]
    pub source_url: Option<String>,
    #[builder(default)]
    pub organization_id: Option<Uuid>,
}

// =============================================================================
// Filter params
// =============================================================================

/// Shared filter parameters for post listing queries.
/// All fields are optional — default is no filtering.
#[derive(Debug, Clone, Default)]
pub struct PostFilters<'a> {
    pub status: Option<&'a str>,
    pub source_type: Option<&'a str>,
    pub source_id: Option<Uuid>,
    pub search: Option<&'a str>,
    pub post_type: Option<&'a str>,
    pub submission_type: Option<&'a str>,
    pub exclude_submission_type: Option<&'a str>,
    pub county_id: Option<Uuid>,
    pub statewide_only: bool,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Post {
    /// SQL predicate: filters out posts with all-expired schedules.
    /// Posts without schedules (evergreen) always pass.
    /// Posts with at least one active schedule pass.
    ///
    /// Schedule "active" means:
    /// - One-off event: dtend (or dtstart if no dtend) is in the future
    /// - Recurring/operating hours: valid_to is NULL (open-ended) or in the future
    const SCHEDULE_ACTIVE_FILTER: &'static str = r#"
        AND (
            NOT EXISTS (
                SELECT 1 FROM schedules s
                WHERE s.schedulable_type = 'post' AND s.schedulable_id = p.id
            )
            OR EXISTS (
                SELECT 1 FROM schedules s
                WHERE s.schedulable_type = 'post' AND s.schedulable_id = p.id
                AND (
                    (NULLIF(s.rrule, '') IS NULL AND COALESCE(s.dtend, s.dtstart) > NOW())
                    OR (NULLIF(s.rrule, '') IS NOT NULL AND (s.valid_to IS NULL OR s.valid_to >= CURRENT_DATE))
                )
            )
        )
    "#;

    /// Batch-load posts by IDs (for DataLoader)
    pub async fn find_by_ids(ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM posts WHERE id = ANY($1) AND deleted_at IS NULL")
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
    /// When source_type/source_id are provided, filters via JOIN through post_sources.
    /// When search is provided, uses full-text search (tsvector) with trigram fallback on title.
    pub async fn find_paginated(
        filters: &PostFilters<'_>,
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();
        let search_term = filters.search;

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    r#"
                    SELECT DISTINCT p.* FROM posts p
                    LEFT JOIN post_sources ps ON ps.post_id = p.id
                    WHERE ($1::text IS NULL OR p.status = $1)
                      AND p.deleted_at IS NULL
                      AND p.revision_of_post_id IS NULL
                      AND p.translation_of_id IS NULL
                      AND ($2::uuid IS NULL OR p.id > $2)
                      AND ($4::text IS NULL OR ps.source_type = $4)
                      AND ($5::uuid IS NULL OR ps.source_id = $5)
                      AND ($6::text IS NULL OR (
                          p.search_vector @@ websearch_to_tsquery('english', $6)
                          OR p.title %> $6
                      ))
                      AND ($7::text IS NULL OR p.post_type = $7)
                      AND ($8::text IS NULL OR p.submission_type = $8)
                      AND ($9::uuid IS NULL OR EXISTS (
                          SELECT 1 FROM zip_counties zc
                          WHERE zc.zip_code = p.zip_code AND zc.county_id = $9
                      ))
                      AND ($10::bool IS NOT TRUE OR p.zip_code IS NULL)
                      AND ($11::text IS NULL OR p.submission_type IS DISTINCT FROM $11)
                    ORDER BY p.id ASC
                    LIMIT $3
                    "#,
                )
                .bind(filters.status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(filters.source_type)
                .bind(filters.source_id)
                .bind(search_term)
                .bind(filters.post_type)
                .bind(filters.submission_type)
                .bind(filters.county_id)
                .bind(filters.statewide_only)
                .bind(filters.exclude_submission_type)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                // Fetch in reverse order, then re-sort
                let mut rows = sqlx::query_as::<_, Self>(
                    r#"
                    SELECT DISTINCT p.* FROM posts p
                    LEFT JOIN post_sources ps ON ps.post_id = p.id
                    WHERE ($1::text IS NULL OR p.status = $1)
                      AND p.deleted_at IS NULL
                      AND p.revision_of_post_id IS NULL
                      AND p.translation_of_id IS NULL
                      AND ($2::uuid IS NULL OR p.id < $2)
                      AND ($4::text IS NULL OR ps.source_type = $4)
                      AND ($5::uuid IS NULL OR ps.source_id = $5)
                      AND ($6::text IS NULL OR (
                          p.search_vector @@ websearch_to_tsquery('english', $6)
                          OR p.title %> $6
                      ))
                      AND ($7::text IS NULL OR p.post_type = $7)
                      AND ($8::text IS NULL OR p.submission_type = $8)
                      AND ($9::uuid IS NULL OR EXISTS (
                          SELECT 1 FROM zip_counties zc
                          WHERE zc.zip_code = p.zip_code AND zc.county_id = $9
                      ))
                      AND ($10::bool IS NOT TRUE OR p.zip_code IS NULL)
                      AND ($11::text IS NULL OR p.submission_type IS DISTINCT FROM $11)
                    ORDER BY p.id DESC
                    LIMIT $3
                    "#,
                )
                .bind(filters.status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .bind(filters.source_type)
                .bind(filters.source_id)
                .bind(search_term)
                .bind(filters.post_type)
                .bind(filters.submission_type)
                .bind(filters.county_id)
                .bind(filters.statewide_only)
                .bind(filters.exclude_submission_type)
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

    /// Find active posts for an organization (joins through post_sources → sources)
    pub async fn find_by_organization_id(
        organization_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let sql = format!(
            r#"
            SELECT DISTINCT p.* FROM posts p
            JOIN post_sources ps ON ps.post_id = p.id
            JOIN sources s ON ps.source_id = s.id
            WHERE s.organization_id = $1
              AND p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              {}
            ORDER BY p.created_at DESC
            "#,
            Self::SCHEDULE_ACTIVE_FILTER
        );
        sqlx::query_as::<_, Post>(&sql)
            .bind(organization_id)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find ALL posts for an organization (joins through post_sources → sources), regardless of status.
    /// Used by admin views.
    pub async fn find_all_by_organization_id(
        organization_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Post>(
            r#"
            SELECT DISTINCT p.* FROM posts p
            JOIN post_sources ps ON ps.post_id = p.id
            JOIN sources s ON ps.source_id = s.id
            WHERE s.organization_id = $1
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND NOT EXISTS (
                SELECT 1 FROM sync_proposals sp
                JOIN sync_batches sb ON sb.id = sp.batch_id
                WHERE sp.draft_entity_id = p.id
                  AND sp.status = 'pending'
                  AND sb.status IN ('pending', 'partially_reviewed')
              )
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new post (returns inserted record with defaults applied)
    pub async fn create(input: CreatePost, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (
                title,
                description,
                description_markdown,
                summary,
                post_type,
                category,
                weight,
                priority,
                urgency,
                location,
                status,
                source_language,
                submission_type,
                submitted_by_id,
                source_url,
                revision_of_post_id,
                translation_of_id,
                published_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(input.title)
        .bind(input.description)
        .bind(input.description_markdown)
        .bind(input.summary)
        .bind(input.post_type)
        .bind(input.category)
        .bind(input.weight)
        .bind(input.priority)
        .bind(input.urgency)
        .bind(input.location)
        .bind(input.status)
        .bind(input.source_language)
        .bind(input.submission_type)
        .bind(input.submitted_by_id)
        .bind(input.source_url)
        .bind(input.revision_of_post_id)
        .bind(input.translation_of_id)
        .bind(input.published_at)
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

    /// Update post content (for edit + approve)
    pub async fn update_content(input: UpdatePostContent, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                description_markdown = COALESCE($4, description_markdown),
                summary = COALESCE($5, summary),
                post_type = COALESCE($6, post_type),
                category = COALESCE($7, category),
                weight = COALESCE($8, weight),
                priority = COALESCE($9, priority),
                urgency = CASE WHEN $10 = '' THEN NULL WHEN $10 IS NOT NULL THEN $10 ELSE urgency END,
                location = CASE WHEN $11 = '' THEN NULL WHEN $11 IS NOT NULL THEN $11 ELSE location END,
                zip_code = CASE WHEN $12 = '' THEN NULL WHEN $12 IS NOT NULL THEN $12 ELSE zip_code END,
                source_url = CASE WHEN $13 = '' THEN NULL WHEN $13 IS NOT NULL THEN $13 ELSE source_url END,
                organization_id = COALESCE($14, organization_id),
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
        .bind(input.post_type)
        .bind(input.category)
        .bind(input.weight)
        .bind(input.priority)
        .bind(input.urgency)
        .bind(input.location)
        .bind(input.zip_code)
        .bind(input.source_url)
        .bind(input.organization_id)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }

    /// Mark posts as expired when all their schedules have passed.
    /// Only affects posts that have schedules (evergreen posts are untouched).
    pub async fn expire_by_schedule(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE posts SET status = 'expired', updated_at = NOW()
            WHERE status = 'active'
              AND EXISTS (
                SELECT 1 FROM schedules s
                WHERE s.schedulable_type = 'post' AND s.schedulable_id = posts.id
              )
              AND NOT EXISTS (
                SELECT 1 FROM schedules s
                WHERE s.schedulable_type = 'post' AND s.schedulable_id = posts.id
                AND (
                  (NULLIF(s.rrule, '') IS NULL AND COALESCE(s.dtend, s.dtstart) > NOW())
                  OR (NULLIF(s.rrule, '') IS NOT NULL AND (s.valid_to IS NULL OR s.valid_to >= CURRENT_DATE))
                )
              )
            "#,
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Count listings by status (for pagination)
    /// When source_type/source_id are provided, filters via JOIN through post_sources.
    /// When search is provided, uses full-text search (tsvector) with trigram fallback on title.
    pub async fn count_by_status(filters: &PostFilters<'_>, pool: &PgPool) -> Result<i64> {
        let search_term = filters.search;
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(DISTINCT p.id)
            FROM posts p
            LEFT JOIN post_sources ps ON ps.post_id = p.id
            WHERE ($1::text IS NULL OR p.status = $1)
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($2::text IS NULL OR ps.source_type = $2)
              AND ($3::uuid IS NULL OR ps.source_id = $3)
              AND ($4::text IS NULL OR (
                  p.search_vector @@ websearch_to_tsquery('english', $4)
                  OR p.title % $4
              ))
              AND ($5::text IS NULL OR p.post_type = $5)
              AND ($6::text IS NULL OR p.submission_type = $6)
            "#,
        )
        .bind(filters.status)
        .bind(filters.source_type)
        .bind(filters.source_id)
        .bind(search_term)
        .bind(filters.post_type)
        .bind(filters.submission_type)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Count posts grouped by post_type and submission_type for a given status.
    /// Returns (post_type, submission_type, count) tuples.
    pub async fn stats_by_status(
        status: Option<&str>,
        pool: &PgPool,
    ) -> Result<Vec<(Option<String>, Option<String>, i64)>> {
        sqlx::query_as::<_, (Option<String>, Option<String>, i64)>(
            r#"
            SELECT p.post_type, p.submission_type, COUNT(*)::bigint
            FROM posts p
            WHERE ($1::text IS NULL OR p.status = $1)
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
            GROUP BY p.post_type, p.submission_type
            "#,
        )
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete all posts for an organization (hard delete via post_sources → sources join).
    /// Returns the count of deleted posts.
    pub async fn delete_all_for_organization(organization_id: Uuid, pool: &PgPool) -> Result<i64> {
        let result = sqlx::query(
            r#"
            DELETE FROM posts
            WHERE id IN (
                SELECT DISTINCT p.id FROM posts p
                JOIN post_sources ps ON ps.post_id = p.id
                JOIN sources s ON ps.source_id = s.id
                WHERE s.organization_id = $1
            )
            "#,
        )
        .bind(organization_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }

    /// Delete a listing by ID (hard delete)
    pub async fn delete(id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
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
        let sql = format!(
            r#"
            WITH center AS (
                SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1
            )
            SELECT p.id, p.title, p.description,
                   p.description_markdown, p.summary,
                   p.post_type, p.category, p.status, p.urgency,
                   p.location, p.submission_type, p.source_url,
                   p.created_at, p.published_at, p.updated_at,
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
              {}
            ORDER BY distance_miles ASC
            LIMIT $3
            "#,
            Self::SCHEDULE_ACTIVE_FILTER
        );
        sqlx::query_as::<_, PostWithDistance>(&sql)
            .bind(center_zip)
            .bind(radius_miles)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find posts near a zip code with composable filters and offset pagination.
    /// Uses GROUP BY + MIN(haversine) for multi-location dedup, bounding box pre-filter,
    /// and COUNT(*) OVER() for total count in a single query.
    pub async fn find_paginated_near_zip(
        center_zip: &str,
        radius_miles: f64,
        filters: &PostFilters<'_>,
        limit: i32,
        offset: i32,
        pool: &PgPool,
    ) -> Result<(Vec<PostWithDistanceAndCount>, bool)> {
        let search_term = filters.search;

        let results = sqlx::query_as::<_, PostWithDistanceAndCount>(
            r#"
            WITH center AS (
                SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1
            )
            SELECT p.id, p.title, p.description,
                   p.description_markdown, p.summary,
                   p.post_type, p.category, p.status, p.urgency,
                   p.location, p.submission_type, p.source_url,
                   p.created_at, p.published_at, p.updated_at,
                   MIN(haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude)) as distance_miles,
                   COUNT(*) OVER() as total_count
            FROM posts p
            INNER JOIN post_locations pl ON pl.post_id = p.id
            INNER JOIN locations l ON l.id = pl.location_id
            INNER JOIN zip_codes z ON l.postal_code = z.zip_code
            LEFT JOIN post_sources ps ON ps.post_id = p.id
            CROSS JOIN center c
            WHERE p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($2::text IS NULL OR p.status = $2)
              AND ($3::text IS NULL OR ps.source_type = $3)
              AND ($4::uuid IS NULL OR ps.source_id = $4)
              AND ($5::text IS NULL OR (
                  p.search_vector @@ websearch_to_tsquery('english', $5)
                  OR p.title % $5
              ))
              AND ($9::text IS NULL OR p.post_type = $9)
              AND ($10::text IS NULL OR p.submission_type = $10)
              AND ($11::uuid IS NULL OR EXISTS (
                  SELECT 1 FROM zip_counties zc
                  WHERE zc.zip_code = p.zip_code AND zc.county_id = $11
              ))
              AND z.latitude BETWEEN c.latitude - ($6::float8 / 69.0)
                                 AND c.latitude + ($6::float8 / 69.0)
              AND z.longitude BETWEEN c.longitude - ($6::float8 / (69.0 * cos(radians(c.latitude))))
                                  AND c.longitude + ($6::float8 / (69.0 * cos(radians(c.latitude))))
            GROUP BY p.id, p.title, p.description, p.description_markdown, p.summary,
                     p.post_type, p.category, p.status, p.urgency, p.location,
                     p.submission_type, p.source_url, p.created_at, p.published_at, p.updated_at,
                     c.latitude, c.longitude
            HAVING MIN(haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude)) <= $6
            ORDER BY distance_miles ASC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(center_zip)             // $1
        .bind(filters.status)         // $2
        .bind(filters.source_type)    // $3
        .bind(filters.source_id)      // $4
        .bind(search_term)            // $5
        .bind(radius_miles)           // $6
        .bind(limit + 1)             // $7 - fetch one extra to detect next page
        .bind(offset)                // $8
        .bind(filters.post_type)      // $9
        .bind(filters.submission_type) // $10
        .bind(filters.county_id)      // $11
        .fetch_all(pool)
        .await?;

        let has_more = results.len() > limit as usize;
        let results = if has_more {
            results.into_iter().take(limit as usize).collect()
        } else {
            results
        };

        Ok((results, has_more))
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

    /// Find revisions by source (for bulk operations)
    pub async fn find_revisions_by_source(
        source_type: &str,
        source_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Post>(
            r#"
            SELECT p.* FROM posts p
            JOIN post_sources ps ON ps.post_id = p.id
            WHERE p.revision_of_post_id IS NOT NULL
              AND ps.source_type = $1 AND ps.source_id = $2
              AND p.deleted_at IS NULL
              AND p.status = 'pending_approval'
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    // =========================================================================
    // Event Schedule Queries (joins against tags)
    // =========================================================================

    /// Find schedules for event-type posts.
    /// Used by the upcoming_events query.
    pub async fn find_event_schedules(pool: &PgPool) -> Result<Vec<Schedule>> {
        sqlx::query_as::<_, Schedule>(
            r#"
            SELECT s.* FROM schedules s
            INNER JOIN posts p ON p.id = s.schedulable_id
            WHERE s.schedulable_type = 'post'
              AND p.post_type = 'event'
              AND p.deleted_at IS NULL
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    // =========================================================================
    // Public Filtered Queries (for home page directory)
    // =========================================================================

    /// Find active posts with optional post_type column and category tag filters.
    ///
    /// - `post_type`: the post_type column value ('story', 'notice', 'exchange', etc.)
    /// - `category`: a `service_offered` tag value like "food-assistance", "legal-aid"
    pub async fn find_public_filtered(
        post_type: Option<&str>,
        category: Option<&str>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let sql = format!(
            r#"
            SELECT DISTINCT p.* FROM posts p
            LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
            LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($1::text IS NULL OR p.post_type = $1)
              AND ($2::text IS NULL OR t_cat.value = $2)
              {}
            ORDER BY p.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            Self::SCHEDULE_ACTIVE_FILTER
        );
        sqlx::query_as::<_, Self>(&sql)
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
        let sql = format!(
            r#"
            SELECT COUNT(DISTINCT p.id) FROM posts p
            LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
            LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND ($1::text IS NULL OR p.post_type = $1)
              AND ($2::text IS NULL OR t_cat.value = $2)
              {}
            "#,
            Self::SCHEDULE_ACTIVE_FILTER
        );
        sqlx::query_scalar::<_, i64>(&sql)
            .bind(post_type)
            .bind(category)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Find active posts near a zip code with optional post_type and category filters.
    /// Returns posts ordered by distance, with distance_miles included.
    pub async fn find_public_filtered_near_zip(
        zip_code: &str,
        radius_miles: f64,
        post_type: Option<&str>,
        category: Option<&str>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<PostWithDistance>> {
        let sql = format!(
            r#"
            WITH center AS (
                SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1
            )
            SELECT DISTINCT ON (p.id)
                   p.id, p.title, p.description,
                   p.description_markdown, p.summary,
                   p.post_type, p.category, p.status, p.urgency,
                   p.location, p.submission_type, p.source_url,
                   p.created_at, p.published_at, p.updated_at,
                   l.postal_code as zip_code, l.city as location_city,
                   haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude) as distance_miles
            FROM posts p
            INNER JOIN post_locations pl ON pl.post_id = p.id
            INNER JOIN locations l ON l.id = pl.location_id
            INNER JOIN zip_codes z ON l.postal_code = z.zip_code
            CROSS JOIN center c
            LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
            LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude) <= $2
              AND ($3::text IS NULL OR p.post_type = $3)
              AND ($4::text IS NULL OR t_cat.value = $4)
              {}
            ORDER BY p.id, distance_miles ASC
            "#,
            Self::SCHEDULE_ACTIVE_FILTER
        );
        // Wrap in a subquery to sort by distance and apply limit/offset
        let wrapped = format!(
            "SELECT * FROM ({}) sub ORDER BY distance_miles ASC LIMIT $5 OFFSET $6",
            sql
        );
        sqlx::query_as::<_, PostWithDistance>(&wrapped)
            .bind(zip_code)
            .bind(radius_miles)
            .bind(post_type)
            .bind(category)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Count active posts near a zip code with optional post_type/category filters.
    pub async fn count_public_filtered_near_zip(
        zip_code: &str,
        radius_miles: f64,
        post_type: Option<&str>,
        category: Option<&str>,
        pool: &PgPool,
    ) -> Result<i64> {
        let sql = format!(
            r#"
            WITH center AS (
                SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1
            )
            SELECT COUNT(DISTINCT p.id) FROM posts p
            INNER JOIN post_locations pl ON pl.post_id = p.id
            INNER JOIN locations l ON l.id = pl.location_id
            INNER JOIN zip_codes z ON l.postal_code = z.zip_code
            CROSS JOIN center c
            LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
            LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
              AND haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude) <= $2
              AND ($3::text IS NULL OR p.post_type = $3)
              AND ($4::text IS NULL OR t_cat.value = $4)
              {}
            "#,
            Self::SCHEDULE_ACTIVE_FILTER
        );
        sqlx::query_scalar::<_, i64>(&sql)
            .bind(zip_code)
            .bind(radius_miles)
            .bind(post_type)
            .bind(category)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Cross-Source Deduplication
    // =========================================================================

    /// Mark a post as a duplicate of another post.
    /// Sets duplicate_of_id, soft-deletes, and records the reason.
    pub async fn mark_as_duplicate(
        id: PostId,
        canonical_id: PostId,
        reason: &str,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE posts
            SET duplicate_of_id = $2,
                deleted_at = NOW(),
                deleted_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(canonical_id)
        .bind(reason)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find organization names for multiple posts (via post_sources → sources → organizations).
    /// Returns a map of post_id → organization name.
    /// Find organization id and name for multiple posts (via post_sources → sources → organizations).
    /// Returns a map of post_id → (org_id, org_name).
    pub async fn find_org_info_for_posts(
        post_ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<Uuid, (Uuid, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String)>(
            r#"
            SELECT DISTINCT ON (p.id) p.id, o.id, o.name
            FROM posts p
            JOIN post_sources ps ON ps.post_id = p.id
            JOIN sources s ON ps.source_id = s.id
            JOIN organizations o ON s.organization_id = o.id
            WHERE p.id = ANY($1)
            ORDER BY p.id
            "#,
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|(post_id, org_id, org_name)| (post_id, (org_id, org_name))).collect())
    }

    /// Find organization name for a post (via post_sources → sources → organizations).
    /// Returns None if the post has no linked organization.
    pub async fn find_org_name(id: PostId, pool: &PgPool) -> Result<Option<String>> {
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT o.name
            FROM posts p
            JOIN post_sources ps ON ps.post_id = p.id
            JOIN sources s ON ps.source_id = s.id
            JOIN organizations o ON s.organization_id = o.id
            WHERE p.id = $1
            LIMIT 1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}
