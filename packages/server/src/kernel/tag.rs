use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{DomainId, ListingId, OrganizationId, TagId, TaggableId};

/// Universal tag - can be associated with any entity via taggables
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: TagId,
    pub kind: String,                 // 'community_served', 'service_area', 'population', etc.
    pub value: String,                // 'somali', 'minneapolis', 'seniors', etc.
    pub display_name: Option<String>, // 'Somali', 'Minneapolis', 'Seniors', etc.
    pub created_at: DateTime<Utc>,
}

/// Polymorphic taggable - links tags to any entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Taggable {
    pub id: TaggableId,
    pub tag_id: TagId,
    pub taggable_type: String, // 'listing', 'organization', 'referral_document', 'domain'
    pub taggable_id: Uuid,
    pub added_at: DateTime<Utc>,
}

/// Tag kind enum for type-safe querying
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    CommunityServed,
    ServiceArea,
    Population,
    OrgLeadership,
    VerificationSource,
}

impl std::fmt::Display for TagKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagKind::CommunityServed => write!(f, "community_served"),
            TagKind::ServiceArea => write!(f, "service_area"),
            TagKind::Population => write!(f, "population"),
            TagKind::OrgLeadership => write!(f, "org_leadership"),
            TagKind::VerificationSource => write!(f, "verification_source"),
        }
    }
}

impl std::str::FromStr for TagKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "community_served" => Ok(TagKind::CommunityServed),
            "service_area" => Ok(TagKind::ServiceArea),
            "population" => Ok(TagKind::Population),
            "org_leadership" => Ok(TagKind::OrgLeadership),
            "verification_source" => Ok(TagKind::VerificationSource),
            _ => Err(anyhow::anyhow!("Invalid tag kind: {}", s)),
        }
    }
}

/// Taggable type enum for type-safe querying
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaggableType {
    Listing,
    Organization,
    ReferralDocument,
    Domain,
}

impl std::fmt::Display for TaggableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaggableType::Listing => write!(f, "listing"),
            TaggableType::Organization => write!(f, "organization"),
            TaggableType::ReferralDocument => write!(f, "referral_document"),
            TaggableType::Domain => write!(f, "domain"),
        }
    }
}

impl std::str::FromStr for TaggableType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "listing" => Ok(TaggableType::Listing),
            "organization" => Ok(TaggableType::Organization),
            "referral_document" => Ok(TaggableType::ReferralDocument),
            "domain" => Ok(TaggableType::Domain),
            _ => Err(anyhow::anyhow!("Invalid taggable type: {}", s)),
        }
    }
}

// =============================================================================
// Tag Queries
// =============================================================================

impl Tag {
    /// Find tag by ID
    pub async fn find_by_id(id: TagId, pool: &PgPool) -> Result<Self> {
        let tag = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(tag)
    }

    /// Find or create tag by kind and value
    pub async fn find_or_create(
        kind: &str,
        value: &str,
        display_name: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (kind, value, display_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (kind, value) DO UPDATE SET display_name = COALESCE(EXCLUDED.display_name, tags.display_name)
            RETURNING *
            "#,
        )
        .bind(kind)
        .bind(value)
        .bind(display_name)
        .fetch_one(pool)
        .await?;
        Ok(tag)
    }

    /// Find tag by kind and value
    pub async fn find_by_kind_value(kind: &str, value: &str, pool: &PgPool) -> Result<Option<Self>> {
        let tag = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE kind = $1 AND value = $2")
            .bind(kind)
            .bind(value)
            .fetch_optional(pool)
            .await?;
        Ok(tag)
    }

    /// Find all tags of a specific kind
    pub async fn find_by_kind(kind: &str, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE kind = $1 ORDER BY value")
            .bind(kind)
            .fetch_all(pool)
            .await?;
        Ok(tags)
    }

    /// Find all tags for a listing
    pub async fn find_for_listing(listing_id: ListingId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'listing' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(listing_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all tags for an organization
    pub async fn find_for_organization(organization_id: OrganizationId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'organization' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(organization_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all tags for a domain
    pub async fn find_for_domain(domain_id: DomainId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'domain' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(domain_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all tags for a referral document
    pub async fn find_for_document(document_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'referral_document' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(document_id)
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Count tags
    pub async fn count(pool: &PgPool) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags")
            .fetch_one(pool)
            .await?;
        Ok(count)
    }

    /// Delete a tag
    pub async fn delete(id: TagId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// =============================================================================
// Taggable Queries
// =============================================================================

impl Taggable {
    /// Associate a tag with a listing
    pub async fn create_listing_tag(
        listing_id: ListingId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "listing", listing_id.as_uuid(), pool).await
    }

    /// Associate a tag with an organization
    pub async fn create_organization_tag(
        organization_id: OrganizationId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "organization", organization_id.as_uuid(), pool).await
    }

    /// Associate a tag with a domain
    pub async fn create_domain_tag(
        domain_id: DomainId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "domain", domain_id.as_uuid(), pool).await
    }

    /// Associate a tag with a referral document
    pub async fn create_document_tag(
        document_id: Uuid,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "referral_document", &document_id, pool).await
    }

    /// Generic create method
    async fn create(
        tag_id: TagId,
        taggable_type: &str,
        taggable_id: &Uuid,
        pool: &PgPool,
    ) -> Result<Self> {
        let taggable = sqlx::query_as::<_, Taggable>(
            r#"
            INSERT INTO taggables (tag_id, taggable_type, taggable_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (tag_id, taggable_type, taggable_id) DO UPDATE
            SET tag_id = EXCLUDED.tag_id
            RETURNING *
            "#,
        )
        .bind(tag_id)
        .bind(taggable_type)
        .bind(taggable_id)
        .fetch_one(pool)
        .await?;
        Ok(taggable)
    }

    /// Remove a tag from a listing
    pub async fn delete_listing_tag(
        listing_id: ListingId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "listing", listing_id.as_uuid(), pool).await
    }

    /// Remove a tag from an organization
    pub async fn delete_organization_tag(
        organization_id: OrganizationId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "organization", organization_id.as_uuid(), pool).await
    }

    /// Remove a tag from a domain
    pub async fn delete_domain_tag(
        domain_id: DomainId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "domain", domain_id.as_uuid(), pool).await
    }

    /// Remove a tag from a referral document
    pub async fn delete_document_tag(document_id: Uuid, tag_id: TagId, pool: &PgPool) -> Result<()> {
        Self::delete(tag_id, "referral_document", &document_id, pool).await
    }

    /// Generic delete method
    async fn delete(
        tag_id: TagId,
        taggable_type: &str,
        taggable_id: &Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM taggables WHERE tag_id = $1 AND taggable_type = $2 AND taggable_id = $3",
        )
        .bind(tag_id)
        .bind(taggable_type)
        .bind(taggable_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Remove all tags from a listing
    pub async fn delete_all_for_listing(listing_id: ListingId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM taggables WHERE taggable_type = 'listing' AND taggable_id = $1")
            .bind(listing_id.as_uuid())
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Remove all tags from an organization
    pub async fn delete_all_for_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query("DELETE FROM taggables WHERE taggable_type = 'organization' AND taggable_id = $1")
            .bind(organization_id.as_uuid())
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all listings with a specific tag
    pub async fn find_listings_with_tag(tag_id: TagId, pool: &PgPool) -> Result<Vec<Uuid>> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT taggable_id FROM taggables WHERE tag_id = $1 AND taggable_type = 'listing'",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    /// Find all organizations with a specific tag
    pub async fn find_organizations_with_tag(tag_id: TagId, pool: &PgPool) -> Result<Vec<Uuid>> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT taggable_id FROM taggables WHERE tag_id = $1 AND taggable_type = 'organization'",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }
}
