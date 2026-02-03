use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    MemberId, PaginationDirection, ProviderId, ValidatedPaginationArgs, WebsiteId,
};

/// Provider status enum for type-safe querying
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    PendingReview,
    Approved,
    Rejected,
    Suspended,
}

impl std::fmt::Display for ProviderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderStatus::PendingReview => write!(f, "pending_review"),
            ProviderStatus::Approved => write!(f, "approved"),
            ProviderStatus::Rejected => write!(f, "rejected"),
            ProviderStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for ProviderStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending_review" => Ok(ProviderStatus::PendingReview),
            "approved" => Ok(ProviderStatus::Approved),
            "rejected" => Ok(ProviderStatus::Rejected),
            "suspended" => Ok(ProviderStatus::Suspended),
            _ => Err(anyhow::anyhow!("Invalid provider status: {}", s)),
        }
    }
}

/// Provider model - professionals in the provider directory
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Provider {
    pub id: ProviderId,

    // Profile
    pub name: String,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,

    // Links
    pub member_id: Option<Uuid>,
    pub website_id: Option<Uuid>,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,

    // Service modes
    pub offers_in_person: bool,
    pub offers_remote: bool,

    // Availability
    pub accepting_clients: bool,

    // Approval workflow
    pub status: String,
    pub submitted_by: Option<Uuid>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a new provider
#[derive(Debug, Clone)]
pub struct CreateProvider {
    pub name: String,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,
    pub member_id: Option<MemberId>,
    pub website_id: Option<WebsiteId>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,
    pub offers_in_person: bool,
    pub offers_remote: bool,
    pub accepting_clients: bool,
    pub submitted_by: Option<MemberId>,
}

/// Input for updating a provider
#[derive(Debug, Clone, Default)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,
    pub offers_in_person: Option<bool>,
    pub offers_remote: Option<bool>,
    pub accepting_clients: Option<bool>,
}

impl Provider {
    /// Find provider by ID
    pub async fn find_by_id(id: ProviderId, pool: &PgPool) -> Result<Self> {
        let provider = sqlx::query_as::<_, Self>("SELECT * FROM providers WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(provider)
    }

    /// Find provider by ID, returning None if not found
    pub async fn find_by_id_optional(id: ProviderId, pool: &PgPool) -> Result<Option<Self>> {
        let provider = sqlx::query_as::<_, Self>("SELECT * FROM providers WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(provider)
    }

    /// Find provider by member ID
    pub async fn find_by_member_id(member_id: MemberId, pool: &PgPool) -> Result<Option<Self>> {
        let provider = sqlx::query_as::<_, Self>("SELECT * FROM providers WHERE member_id = $1")
            .bind(member_id.as_uuid())
            .fetch_optional(pool)
            .await?;
        Ok(provider)
    }

    /// Find all providers with a specific status
    pub async fn find_by_status(status: &str, pool: &PgPool) -> Result<Vec<Self>> {
        let providers = sqlx::query_as::<_, Self>(
            "SELECT * FROM providers WHERE status = $1 ORDER BY created_at DESC",
        )
        .bind(status)
        .fetch_all(pool)
        .await?;
        Ok(providers)
    }

    /// Find all pending providers (for admin approval queue)
    pub async fn find_pending(pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_by_status("pending_review", pool).await
    }

    /// Find all approved providers
    pub async fn find_approved(pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_by_status("approved", pool).await
    }

    /// Find approved providers that are accepting clients
    pub async fn find_accepting_clients(pool: &PgPool) -> Result<Vec<Self>> {
        let providers = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM providers
            WHERE status = 'approved' AND accepting_clients = true
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(providers)
    }

    /// Find providers with optional filters
    pub async fn find_with_filters(
        status: Option<&str>,
        accepting_clients: Option<bool>,
        limit: Option<i32>,
        offset: Option<i32>,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let mut query = String::from("SELECT * FROM providers WHERE 1=1");
        let mut params: Vec<String> = vec![];
        let mut param_idx = 1;

        if let Some(s) = status {
            query.push_str(&format!(" AND status = ${}", param_idx));
            params.push(s.to_string());
            param_idx += 1;
        }

        if let Some(ac) = accepting_clients {
            query.push_str(&format!(" AND accepting_clients = ${}", param_idx));
            params.push(ac.to_string());
            param_idx += 1;
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT ${}", param_idx));
            params.push(lim.to_string());
            param_idx += 1;
        }

        if let Some(off) = offset {
            query.push_str(&format!(" OFFSET ${}", param_idx));
            params.push(off.to_string());
        }

        // Build query dynamically based on params
        let mut sql_query = sqlx::query_as::<_, Self>(&query);
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        let providers = sql_query.fetch_all(pool).await?;
        Ok(providers)
    }

    /// Create a new provider
    pub async fn create(input: CreateProvider, pool: &PgPool) -> Result<Self> {
        let provider = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO providers (
                name, bio, why_statement, headline, profile_image_url,
                member_id, website_id, location, latitude, longitude,
                service_radius_km, offers_in_person, offers_remote,
                accepting_clients, submitted_by, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 'pending_review')
            RETURNING *
            "#,
        )
        .bind(&input.name)
        .bind(&input.bio)
        .bind(&input.why_statement)
        .bind(&input.headline)
        .bind(&input.profile_image_url)
        .bind(input.member_id.map(|id| *id.as_uuid()))
        .bind(input.website_id.map(|id| *id.as_uuid()))
        .bind(&input.location)
        .bind(input.latitude)
        .bind(input.longitude)
        .bind(input.service_radius_km)
        .bind(input.offers_in_person)
        .bind(input.offers_remote)
        .bind(input.accepting_clients)
        .bind(input.submitted_by.map(|id| *id.as_uuid()))
        .fetch_one(pool)
        .await?;
        Ok(provider)
    }

    /// Update a provider
    pub async fn update(id: ProviderId, input: UpdateProvider, pool: &PgPool) -> Result<Self> {
        let provider = sqlx::query_as::<_, Self>(
            r#"
            UPDATE providers SET
                name = COALESCE($2, name),
                bio = COALESCE($3, bio),
                why_statement = COALESCE($4, why_statement),
                headline = COALESCE($5, headline),
                profile_image_url = COALESCE($6, profile_image_url),
                location = COALESCE($7, location),
                latitude = COALESCE($8, latitude),
                longitude = COALESCE($9, longitude),
                service_radius_km = COALESCE($10, service_radius_km),
                offers_in_person = COALESCE($11, offers_in_person),
                offers_remote = COALESCE($12, offers_remote),
                accepting_clients = COALESCE($13, accepting_clients),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.bio)
        .bind(&input.why_statement)
        .bind(&input.headline)
        .bind(&input.profile_image_url)
        .bind(&input.location)
        .bind(input.latitude)
        .bind(input.longitude)
        .bind(input.service_radius_km)
        .bind(input.offers_in_person)
        .bind(input.offers_remote)
        .bind(input.accepting_clients)
        .fetch_one(pool)
        .await?;
        Ok(provider)
    }

    /// Approve a provider
    pub async fn approve(id: ProviderId, reviewed_by: MemberId, pool: &PgPool) -> Result<Self> {
        let provider = sqlx::query_as::<_, Self>(
            r#"
            UPDATE providers SET
                status = 'approved',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by.as_uuid())
        .fetch_one(pool)
        .await?;
        Ok(provider)
    }

    /// Reject a provider
    pub async fn reject(
        id: ProviderId,
        reviewed_by: MemberId,
        reason: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let provider = sqlx::query_as::<_, Self>(
            r#"
            UPDATE providers SET
                status = 'rejected',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by.as_uuid())
        .bind(reason)
        .fetch_one(pool)
        .await?;
        Ok(provider)
    }

    /// Suspend a provider
    pub async fn suspend(
        id: ProviderId,
        reviewed_by: MemberId,
        reason: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let provider = sqlx::query_as::<_, Self>(
            r#"
            UPDATE providers SET
                status = 'suspended',
                reviewed_by = $2,
                reviewed_at = NOW(),
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewed_by.as_uuid())
        .bind(reason)
        .fetch_one(pool)
        .await?;
        Ok(provider)
    }

    /// Update provider embedding
    pub async fn update_embedding(id: ProviderId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE providers SET embedding = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count providers by status
    pub async fn count_by_status(status: &str, pool: &PgPool) -> Result<i64> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM providers WHERE status = $1")
                .bind(status)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Count all providers
    pub async fn count(pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM providers")
            .fetch_one(pool)
            .await?;
        Ok(count)
    }

    /// Delete a provider
    pub async fn delete(id: ProviderId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM providers WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Cursor-Based Pagination (Relay spec)
    // =========================================================================

    /// Find providers with cursor-based pagination
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
                    SELECT * FROM providers
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
                    SELECT * FROM providers
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

    /// Count providers with optional status filter
    pub async fn count_with_filters(status: Option<&str>, pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM providers WHERE ($1::text IS NULL OR status = $1)",
        )
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}

// Implement Readable for ReadResult<Provider> support
use crate::common::Readable;
use async_trait::async_trait;

#[async_trait]
impl Readable for Provider {
    type Id = ProviderId;

    async fn read_by_id(id: Self::Id, pool: &PgPool) -> Result<Option<Self>> {
        Self::find_by_id_optional(id, pool).await
    }
}
