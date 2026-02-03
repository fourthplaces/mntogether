//! Resource model - extracted services/programs from websites
//!
//! Resources are the simplified content model that replaces the complex Listing model.
//! Each resource represents a distinct service, program, or opportunity extracted
//! from a website's pages.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{PaginationDirection, ResourceId, ValidatedPaginationArgs, WebsiteId};

/// Resource status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    PendingApproval,
    Active,
    Rejected,
    Expired,
}

impl std::fmt::Display for ResourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceStatus::PendingApproval => write!(f, "pending_approval"),
            ResourceStatus::Active => write!(f, "active"),
            ResourceStatus::Rejected => write!(f, "rejected"),
            ResourceStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for ResourceStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_approval" => Ok(ResourceStatus::PendingApproval),
            "active" => Ok(ResourceStatus::Active),
            "rejected" => Ok(ResourceStatus::Rejected),
            "expired" => Ok(ResourceStatus::Expired),
            _ => Err(anyhow::anyhow!("Invalid resource status: {}", s)),
        }
    }
}

/// Resource - a service, program, or opportunity extracted from a website
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Resource {
    pub id: ResourceId,
    pub website_id: WebsiteId,

    // Core content
    pub title: String,
    pub content: String,

    // Location/service area
    pub location: Option<String>,

    // Workflow status
    pub status: String,

    // Source tracking
    pub organization_name: Option<String>,

    // Vector search (for semantic matching and deduplication)
    pub embedding: Option<pgvector::Vector>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Resource {
    /// Find resource by ID
    pub async fn find_by_id(id: ResourceId, pool: &PgPool) -> Result<Self> {
        let resource = sqlx::query_as::<_, Self>("SELECT * FROM resources WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(resource)
    }

    /// Find resource by ID (optional)
    pub async fn find_by_id_optional(id: ResourceId, pool: &PgPool) -> Result<Option<Self>> {
        let resource = sqlx::query_as::<_, Self>("SELECT * FROM resources WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(resource)
    }

    /// Find resources by website
    pub async fn find_by_website_id(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let resources = sqlx::query_as::<_, Self>(
            "SELECT * FROM resources WHERE website_id = $1 ORDER BY created_at DESC",
        )
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(resources)
    }

    /// Find resources by status
    pub async fn find_by_status(
        status: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let resources = sqlx::query_as::<_, Self>(
            "SELECT * FROM resources
             WHERE status = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(resources)
    }

    /// Count resources by status
    pub async fn count_by_status(status: &str, pool: &PgPool) -> Result<i64> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM resources WHERE status = $1")
                .bind(status)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Find resources with cursor-based pagination (Relay spec)
    ///
    /// Uses V7 UUID ordering (time-based) for stable pagination.
    /// Fetches limit+1 to detect if there are more pages.
    pub async fn find_paginated(
        status: Option<&str>,
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    r#"
                    SELECT * FROM resources
                    WHERE ($1::text IS NULL OR status = $1)
                      AND ($2::uuid IS NULL OR id > $2)
                    ORDER BY id ASC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                let mut rows = sqlx::query_as::<_, Self>(
                    r#"
                    SELECT * FROM resources
                    WHERE ($1::text IS NULL OR status = $1)
                      AND ($2::uuid IS NULL OR id < $2)
                    ORDER BY id DESC
                    LIMIT $3
                    "#,
                )
                .bind(status)
                .bind(args.cursor)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await?;

                rows.reverse();
                rows
            }
        };

        let has_more = results.len() > args.limit as usize;
        let results = if has_more {
            results.into_iter().take(args.limit as usize).collect()
        } else {
            results
        };

        Ok((results, has_more))
    }

    /// Count resources with optional status filter
    pub async fn count_with_filters(status: Option<ResourceStatus>, pool: &PgPool) -> Result<i64> {
        let status_str = status.map(|s| s.to_string());
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resources WHERE ($1::text IS NULL OR status = $1)",
        )
        .bind(status_str)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Find active resources for a website
    pub async fn find_active_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let resources = sqlx::query_as::<_, Self>(
            "SELECT * FROM resources
             WHERE website_id = $1
               AND status IN ('pending_approval', 'active')
             ORDER BY created_at DESC",
        )
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(resources)
    }

    /// Create a new resource
    pub async fn create(
        website_id: WebsiteId,
        title: String,
        content: String,
        location: Option<String>,
        organization_name: Option<String>,
        status: ResourceStatus,
        pool: &PgPool,
    ) -> Result<Self> {
        let resource = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO resources (website_id, title, content, location, organization_name, status)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(website_id)
        .bind(title)
        .bind(content)
        .bind(location)
        .bind(organization_name)
        .bind(status.to_string())
        .fetch_one(pool)
        .await?;
        Ok(resource)
    }

    /// Update resource content
    pub async fn update_content(
        id: ResourceId,
        title: Option<String>,
        content: Option<String>,
        location: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let resource = sqlx::query_as::<_, Self>(
            r#"
            UPDATE resources
            SET
                title = COALESCE($2, title),
                content = COALESCE($3, content),
                location = COALESCE($4, location),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(content)
        .bind(location)
        .fetch_one(pool)
        .await?;
        Ok(resource)
    }

    /// Update resource status
    pub async fn update_status(
        id: ResourceId,
        status: ResourceStatus,
        pool: &PgPool,
    ) -> Result<Self> {
        let resource = sqlx::query_as::<_, Self>(
            r#"
            UPDATE resources
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status.to_string())
        .fetch_one(pool)
        .await?;
        Ok(resource)
    }

    /// Update resource embedding
    pub async fn update_embedding(id: ResourceId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE resources SET embedding = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Find similar resources by embedding (for deduplication pre-filter)
    ///
    /// Returns resources with cosine similarity above the threshold, ordered by similarity.
    /// This is used to find candidate duplicates before AI comparison.
    pub async fn find_similar_by_embedding(
        embedding: &[f32],
        website_id: WebsiteId,
        threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<(Self, f32)>> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        // Use cosine distance (1 - cosine_similarity), so lower is more similar
        // We want similarity > threshold, which means distance < (1 - threshold)
        let max_distance = 1.0 - threshold;

        // First get IDs and similarity scores
        let rows: Vec<(ResourceId, f32)> = sqlx::query_as(
            r#"
            SELECT r.id, (1 - (r.embedding <=> $1))::float4 as similarity
            FROM resources r
            WHERE r.website_id = $2
              AND r.embedding IS NOT NULL
              AND r.status IN ('pending_approval', 'active')
              AND (r.embedding <=> $1) < $3
            ORDER BY r.embedding <=> $1
            LIMIT $4
            "#,
        )
        .bind(vector)
        .bind(website_id)
        .bind(max_distance)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        // Then fetch full resources
        let mut results = Vec::with_capacity(rows.len());
        for (id, similarity) in rows {
            if let Ok(resource) = Self::find_by_id(id, pool).await {
                results.push((resource, similarity));
            }
        }

        Ok(results)
    }

    /// Find similar resources across all websites (for global deduplication)
    pub async fn find_similar_globally(
        embedding: &[f32],
        threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<(Self, f32)>> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());
        let max_distance = 1.0 - threshold;

        // First get IDs and similarity scores
        let rows: Vec<(ResourceId, f32)> = sqlx::query_as(
            r#"
            SELECT r.id, (1 - (r.embedding <=> $1))::float4 as similarity
            FROM resources r
            WHERE r.embedding IS NOT NULL
              AND r.status IN ('pending_approval', 'active')
              AND (r.embedding <=> $1) < $2
            ORDER BY r.embedding <=> $1
            LIMIT $3
            "#,
        )
        .bind(vector)
        .bind(max_distance)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        // Then fetch full resources
        let mut results = Vec::with_capacity(rows.len());
        for (id, similarity) in rows {
            if let Ok(resource) = Self::find_by_id(id, pool).await {
                results.push((resource, similarity));
            }
        }

        Ok(results)
    }

    /// Delete a resource
    pub async fn delete(id: ResourceId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM resources WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find resources without embeddings
    pub async fn find_without_embeddings(limit: i64, pool: &PgPool) -> Result<Vec<Self>> {
        let resources = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM resources
            WHERE embedding IS NULL
              AND status IN ('pending_approval', 'active')
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(resources)
    }

    /// Count resources without embeddings
    pub async fn count_without_embeddings(pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resources WHERE embedding IS NULL AND status IN ('pending_approval', 'active')",
        )
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Find pending resources (for admin approval queue)
    pub async fn find_pending(pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_by_status("pending_approval", 100, 0, pool).await
    }

    /// Find active resources
    pub async fn find_active(limit: i64, pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_by_status("active", limit, 0, pool).await
    }

    /// Find resources with optional status filter
    pub async fn find_with_filters(
        status: Option<ResourceStatus>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        match status {
            Some(s) => Self::find_by_status(&s.to_string(), limit, offset, pool).await,
            None => Self::find_by_status("pending_approval", limit, offset, pool).await,
        }
    }
}

// Implement Readable for ReadResult<Resource> support
use crate::common::Readable;
use async_trait::async_trait;

#[async_trait]
impl Readable for Resource {
    type Id = ResourceId;

    async fn read_by_id(id: Self::Id, pool: &PgPool) -> Result<Option<Self>> {
        Self::find_by_id_optional(id, pool).await
    }
}
