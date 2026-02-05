use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typed_builder::TypedBuilder;

use crate::common::{OrganizationId, PaginationDirection, ValidatedPaginationArgs, WebsiteId};

// Builder for creating organizations
#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct CreateOrganization {
    pub name: String,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub summary: Option<String>,
    #[builder(default)]
    pub website_id: Option<WebsiteId>,
    #[builder(default)]
    pub website: Option<String>,
    #[builder(default)]
    pub phone: Option<String>,
    #[builder(default)]
    pub email: Option<String>,
    #[builder(default)]
    pub primary_address: Option<String>,
    #[builder(default)]
    pub organization_type: Option<String>,
}

// Builder for updating organizations
#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct UpdateOrganization {
    pub id: OrganizationId,
    #[builder(default)]
    pub name: Option<String>,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub website: Option<String>,
    #[builder(default)]
    pub phone: Option<String>,
    #[builder(default)]
    pub email: Option<String>,
    #[builder(default)]
    pub primary_address: Option<String>,
    #[builder(default)]
    pub organization_type: Option<String>,
}

// Helper struct for similarity search results
#[derive(Debug, sqlx::FromRow)]
struct OrganizationWithSimilarity {
    // Organization fields
    id: OrganizationId,
    name: String,
    description: Option<String>,
    summary: Option<String>,
    website_id: Option<WebsiteId>,
    website: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    primary_address: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    verified: bool,
    verified_at: Option<DateTime<Utc>>,
    claimed_at: Option<DateTime<Utc>>,
    claim_token: Option<String>,
    claim_email: Option<String>,
    organization_type: Option<String>,
    embedding: Option<pgvector::Vector>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,

    // Computed similarity score
    similarity: f32,
}

impl From<OrganizationWithSimilarity> for (Organization, f32) {
    fn from(result: OrganizationWithSimilarity) -> Self {
        let org = Organization {
            id: result.id,
            name: result.name,
            description: result.description,
            summary: result.summary,
            website_id: result.website_id,
            website: result.website,
            phone: result.phone,
            email: result.email,
            primary_address: result.primary_address,
            latitude: result.latitude,
            longitude: result.longitude,
            verified: result.verified,
            verified_at: result.verified_at,
            claimed_at: result.claimed_at,
            claim_token: result.claim_token,
            claim_email: result.claim_email,
            organization_type: result.organization_type,
            embedding: result.embedding,
            created_at: result.created_at,
            updated_at: result.updated_at,
        };
        (org, result.similarity)
    }
}

/// Organization - entity providing services or opportunities
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,
    pub summary: Option<String>, // Rich summary for AI matching (used with description for embeddings)
    pub website_id: Option<WebsiteId>,

    // Contact
    pub website: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,

    // Location
    pub primary_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Verification
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,

    // Claiming (for capacity updates)
    pub claimed_at: Option<DateTime<Utc>>,
    pub claim_token: Option<String>,
    pub claim_email: Option<String>,

    pub organization_type: Option<String>, // 'nonprofit', 'government', 'business', 'community', 'other'

    // Vector search (for semantic matching in chat/referrals)
    pub embedding: Option<pgvector::Vector>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationType {
    Nonprofit,
    Business,
    Community,
    Other,
}

impl std::fmt::Display for OrganizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationType::Nonprofit => write!(f, "nonprofit"),
            OrganizationType::Business => write!(f, "business"),
            OrganizationType::Community => write!(f, "community"),
            OrganizationType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for OrganizationType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "nonprofit" => Ok(OrganizationType::Nonprofit),
            "business" => Ok(OrganizationType::Business),
            "community" => Ok(OrganizationType::Community),
            "other" => Ok(OrganizationType::Other),
            _ => Err(anyhow::anyhow!("Invalid organization type: {}", s)),
        }
    }
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Organization {
    /// Find organization by ID
    pub async fn find_by_id(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(org)
    }

    /// Find organization by name (exact match)
    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await?;
        Ok(org)
    }

    /// Find all verified organizations
    pub async fn find_verified(pool: &PgPool) -> Result<Vec<Self>> {
        let orgs = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE verified = true ORDER BY name",
        )
        .fetch_all(pool)
        .await?;
        Ok(orgs)
    }

    /// Find all organizations
    pub async fn find_all(limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<Self>> {
        let orgs = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations ORDER BY name LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(orgs)
    }

    /// Search organizations by name (fuzzy)
    pub async fn search_by_name(query: &str, pool: &PgPool) -> Result<Vec<Self>> {
        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE name ILIKE $1
            ORDER BY name
            LIMIT 20
            "#,
        )
        .bind(format!("%{}%", query))
        .fetch_all(pool)
        .await?;
        Ok(orgs)
    }

    /// Find organizations by website
    pub async fn find_by_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let orgs = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE website_id = $1 ORDER BY name",
        )
        .bind(website_id)
        .fetch_all(pool)
        .await?;
        Ok(orgs)
    }

    /// Find organization by claim token
    pub async fn find_by_claim_token(claim_token: &str, pool: &PgPool) -> Result<Option<Self>> {
        let org =
            sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE claim_token = $1")
                .bind(claim_token)
                .fetch_optional(pool)
                .await?;
        Ok(org)
    }

    /// Create a new organization using builder pattern
    pub async fn create(builder: CreateOrganization, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (
                name,
                description,
                summary,
                website_id,
                website,
                phone,
                email,
                primary_address,
                organization_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(builder.name)
        .bind(builder.description)
        .bind(builder.summary)
        .bind(builder.website_id)
        .bind(builder.website)
        .bind(builder.phone)
        .bind(builder.email)
        .bind(builder.primary_address)
        .bind(builder.organization_type)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Update organization
    pub async fn update(input: UpdateOrganization, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                website = COALESCE($4, website),
                phone = COALESCE($5, phone),
                email = COALESCE($6, email),
                primary_address = COALESCE($7, primary_address),
                organization_type = COALESCE($8, organization_type),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(input.id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.website)
        .bind(input.phone)
        .bind(input.email)
        .bind(input.primary_address)
        .bind(input.organization_type)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Mark organization as verified
    pub async fn mark_verified(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET verified = true, verified_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Unmark organization as verified
    pub async fn unmark_verified(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET verified = false, verified_at = NULL, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Initiate organization claim (generates claim token)
    pub async fn initiate_claim(
        id: OrganizationId,
        claim_email: String,
        claim_token: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET claim_token = $2, claim_email = $3, updated_at = NOW()
            WHERE id = $1 AND claimed_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(claim_token)
        .bind(claim_email)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Complete organization claim (marks as claimed)
    pub async fn complete_claim(claim_token: &str, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET claimed_at = NOW(), updated_at = NOW()
            WHERE claim_token = $1 AND claimed_at IS NULL
            RETURNING *
            "#,
        )
        .bind(claim_token)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Count organizations
    pub async fn count(pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations")
            .fetch_one(pool)
            .await?;
        Ok(count)
    }

    /// Delete an organization
    pub async fn delete(id: OrganizationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Update organization embedding for semantic search
    pub async fn update_embedding(
        id: OrganizationId,
        embedding: &[f32],
        pool: &PgPool,
    ) -> Result<()> {
        use pgvector::Vector;

        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE organizations SET embedding = $2 WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update summary
    pub async fn update_summary(
        id: OrganizationId,
        summary: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET summary = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(summary)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Search organizations by semantic similarity
    pub async fn search_by_similarity(
        query_embedding: &[f32],
        match_threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<(Self, f32)>> {
        use pgvector::Vector;

        let vector = Vector::from(query_embedding.to_vec());

        // Query organizations and calculate similarity using query_as
        let results: Vec<OrganizationWithSimilarity> = sqlx::query_as(
            r#"
            SELECT
                id, name, description, summary, domain_id,
                website, phone, email, primary_address, latitude, longitude,
                verified, verified_at, claimed_at, claim_token, claim_email,
                organization_type, embedding, created_at, updated_at,
                (1 - (embedding <=> $1)) as similarity
            FROM organizations
            WHERE embedding IS NOT NULL
              AND (1 - (embedding <=> $1)) > $2
            ORDER BY embedding <=> $1
            LIMIT $3
            "#,
        )
        .bind(&vector)
        .bind(match_threshold)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        // Convert to (Organization, f32) tuples
        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Get text for embedding generation (combines description + summary)
    pub fn get_embedding_text(&self) -> String {
        let mut parts = vec![self.name.clone()];

        if let Some(desc) = &self.description {
            parts.push(desc.clone());
        }

        if let Some(summary) = &self.summary {
            parts.push(summary.clone());
        }

        if let Some(org_type) = &self.organization_type {
            parts.push(format!("Type: {}", org_type));
        }

        parts.join(" | ")
    }

    /// Find organizations with cursor-based pagination (Relay spec)
    pub async fn find_paginated(
        args: &ValidatedPaginationArgs,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, bool)> {
        let fetch_limit = args.fetch_limit();

        let results = match args.direction {
            PaginationDirection::Forward => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM organizations WHERE ($1::uuid IS NULL OR id > $1) ORDER BY id ASC LIMIT $2",
                )
                .bind(args.cursor)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await?
            }
            PaginationDirection::Backward => {
                let mut rows = sqlx::query_as::<_, Self>(
                    "SELECT * FROM organizations WHERE ($1::uuid IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
                )
                .bind(args.cursor)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await?;
                rows.reverse();
                rows
            }
        };

        let has_more = results.len() > args.limit as usize;
        let results = results.into_iter().take(args.limit as usize).collect();
        Ok((results, has_more))
    }
}
