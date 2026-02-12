use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ContainerId, PostId, ProviderId, TagId, TaggableId, WebsiteId};

/// Universal tag - can be associated with any entity via taggables
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: TagId,
    pub kind: String,  // 'community_served', 'service_area', 'population', etc.
    pub value: String, // 'somali', 'minneapolis', 'seniors', etc.
    pub display_name: Option<String>, // 'Somali', 'Minneapolis', 'Seniors', etc.
    pub parent_tag_id: Option<TagId>, // Self-referential FK for hierarchy
    pub color: Option<String>, // Optional hex color for display (e.g., '#3b82f6')
    pub description: Option<String>, // Optional description of the tag purpose
    pub emoji: Option<String>, // Optional emoji for display (e.g., '🤲')
    pub created_at: DateTime<Utc>,
}

/// Polymorphic taggable - links tags to any entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Taggable {
    pub id: TaggableId,
    pub tag_id: TagId,
    pub taggable_type: String, // 'post', 'organization', 'referral_document', 'domain', 'provider'
    pub taggable_id: Uuid,
    pub added_at: DateTime<Utc>,
}

/// Taggable type enum for type-safe querying
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaggableType {
    Post,
    ReferralDocument,
    Domain,
    Provider,
    Container,
}

impl std::fmt::Display for TaggableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaggableType::Post => write!(f, "post"),
            TaggableType::ReferralDocument => write!(f, "referral_document"),
            TaggableType::Domain => write!(f, "domain"),
            TaggableType::Provider => write!(f, "provider"),
            TaggableType::Container => write!(f, "container"),
        }
    }
}

impl std::str::FromStr for TaggableType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "post" => Ok(TaggableType::Post),
            "referral_document" => Ok(TaggableType::ReferralDocument),
            "domain" => Ok(TaggableType::Domain),
            "provider" => Ok(TaggableType::Provider),
            "container" => Ok(TaggableType::Container),
            _ => Err(anyhow::anyhow!("Invalid taggable type: {}", s)),
        }
    }
}

/// Active category with post count (for public home page filters)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActiveCategory {
    pub value: String,
    pub display_name: String,
    pub count: i32,
}

/// Helper struct for batch-loading tags with their associated post ID.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TagWithPostId {
    pub taggable_id: Uuid,
    #[sqlx(flatten)]
    pub tag: Tag,
}

// =============================================================================
// Tag Queries
// =============================================================================

impl Tag {
    /// Batch-load tags for multiple posts (for DataLoader).
    /// Returns (post_id, Tag) pairs grouped by the caller.
    pub async fn find_for_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<TagWithPostId>> {
        sqlx::query_as::<_, TagWithPostId>(
            r#"
            SELECT tg.taggable_id, t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'post' AND tg.taggable_id = ANY($1)
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Batch-load only public tags for multiple posts.
    /// Filters by tag_kinds.is_public = true.
    pub async fn find_public_for_post_ids(
        post_ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<Vec<TagWithPostId>> {
        sqlx::query_as::<_, TagWithPostId>(
            r#"
            SELECT tg.taggable_id, t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            INNER JOIN tag_kinds tk ON tk.slug = t.kind
            WHERE tg.taggable_type = 'post'
              AND tg.taggable_id = ANY($1)
              AND tk.is_public = true
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all tags ordered by kind, value
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY kind, value")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Update tag display name
    pub async fn update_display_name(id: TagId, display_name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Tag>("UPDATE tags SET display_name = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(display_name)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Update tag description
    pub async fn update_description(
        id: TagId,
        description: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Tag>("UPDATE tags SET description = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(description)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Update tag emoji
    pub async fn update_emoji(id: TagId, emoji: Option<&str>, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Tag>("UPDATE tags SET emoji = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(emoji)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Update tag color
    pub async fn update_color(id: TagId, color: Option<&str>, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Tag>("UPDATE tags SET color = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(color)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

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
    pub async fn find_by_kind_value(
        kind: &str,
        value: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
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
    pub async fn find_for_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'post' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(post_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all tags for a website
    pub async fn find_for_website(website_id: WebsiteId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'website' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(website_id.as_uuid())
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

    /// Find all tags for a provider
    pub async fn find_for_provider(provider_id: ProviderId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'provider' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(provider_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all tags for a container
    pub async fn find_for_container(container_id: ContainerId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'container' AND tg.taggable_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(container_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Check if container has a specific tag kind/value
    pub async fn container_has_tag(
        container_id: ContainerId,
        kind: &str,
        value: &str,
        pool: &PgPool,
    ) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM tags t
                INNER JOIN taggables tg ON tg.tag_id = t.id
                WHERE tg.taggable_type = 'container'
                  AND tg.taggable_id = $1
                  AND t.kind = $2
                  AND t.value = $3
            )
            "#,
        )
        .bind(container_id.as_uuid())
        .bind(kind)
        .bind(value)
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }

    /// Get the with_agent tag value for a container (if exists)
    pub async fn get_container_agent_config(
        container_id: ContainerId,
        pool: &PgPool,
    ) -> Result<Option<String>> {
        let value = sqlx::query_scalar::<_, String>(
            r#"
            SELECT t.value
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            WHERE tg.taggable_type = 'container'
              AND tg.taggable_id = $1
              AND t.kind = 'with_agent'
            LIMIT 1
            "#,
        )
        .bind(container_id.as_uuid())
        .fetch_optional(pool)
        .await?;
        Ok(value)
    }

    /// Find distinct ServiceOffered tags that are attached to active posts, with counts.
    /// Powers the dynamic category pills on the public home page.
    pub async fn find_active_categories(pool: &PgPool) -> Result<Vec<ActiveCategory>> {
        sqlx::query_as::<_, ActiveCategory>(
            r#"
            SELECT t.value, COALESCE(t.display_name, t.value) as display_name, COUNT(DISTINCT tg.taggable_id)::int as count
            FROM tags t
            INNER JOIN taggables tg ON tg.tag_id = t.id
            INNER JOIN posts p ON p.id = tg.taggable_id
            WHERE t.kind = 'service_offered'
              AND tg.taggable_type = 'post'
              AND p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.translation_of_id IS NULL
            GROUP BY t.value, t.display_name
            HAVING COUNT(DISTINCT tg.taggable_id) > 0
            ORDER BY count DESC, t.value
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all post_type tags for the home page buckets.
    pub async fn find_post_types(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE kind = 'post_type' AND value IN ('seeking', 'offering', 'announcement') ORDER BY CASE value WHEN 'offering' THEN 1 WHEN 'seeking' THEN 2 WHEN 'announcement' THEN 3 END",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
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

    // =========================================================================
    // Hierarchy Queries
    // =========================================================================

    /// Find all child tags of a parent
    pub async fn find_children(parent_id: TagId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE parent_tag_id = $1 ORDER BY kind, value",
        )
        .bind(parent_id)
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find top-level tags (no parent) of a specific kind
    pub async fn find_roots_by_kind(kind: &str, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE kind = $1 AND parent_tag_id IS NULL ORDER BY value",
        )
        .bind(kind)
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }
}

// =============================================================================
// Taggable Queries
// =============================================================================

impl Taggable {
    /// Associate a tag with a post
    pub async fn create_post_tag(post_id: PostId, tag_id: TagId, pool: &PgPool) -> Result<Self> {
        Self::create(tag_id, "post", post_id.as_uuid(), pool).await
    }

    /// Associate a tag with a website
    pub async fn create_website_tag(
        website_id: WebsiteId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "website", website_id.as_uuid(), pool).await
    }

    /// Associate a tag with a referral document
    pub async fn create_document_tag(
        document_id: Uuid,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "referral_document", &document_id, pool).await
    }

    /// Associate a tag with a provider
    pub async fn create_provider_tag(
        provider_id: ProviderId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "provider", provider_id.as_uuid(), pool).await
    }

    /// Associate a tag with a container
    pub async fn create_container_tag(
        container_id: ContainerId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(tag_id, "container", container_id.as_uuid(), pool).await
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

    /// Remove a tag from a post
    pub async fn delete_post_tag(post_id: PostId, tag_id: TagId, pool: &PgPool) -> Result<()> {
        Self::delete(tag_id, "post", post_id.as_uuid(), pool).await
    }

    /// Remove a tag from a website
    pub async fn delete_website_tag(
        website_id: WebsiteId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "website", website_id.as_uuid(), pool).await
    }

    /// Remove a tag from a referral document
    pub async fn delete_document_tag(
        document_id: Uuid,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "referral_document", &document_id, pool).await
    }

    /// Remove a tag from a provider
    pub async fn delete_provider_tag(
        provider_id: ProviderId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "provider", provider_id.as_uuid(), pool).await
    }

    /// Remove a tag from a container
    pub async fn delete_container_tag(
        container_id: ContainerId,
        tag_id: TagId,
        pool: &PgPool,
    ) -> Result<()> {
        Self::delete(tag_id, "container", container_id.as_uuid(), pool).await
    }

    /// Remove all tags from a container
    pub async fn delete_all_for_container(container_id: ContainerId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM taggables WHERE taggable_type = 'container' AND taggable_id = $1")
            .bind(container_id.as_uuid())
            .execute(pool)
            .await?;
        Ok(())
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

    /// Remove all tags from a post
    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM taggables WHERE taggable_type = 'post' AND taggable_id = $1")
            .bind(post_id.as_uuid())
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Remove all tags from a provider
    pub async fn delete_all_for_provider(provider_id: ProviderId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM taggables WHERE taggable_type = 'provider' AND taggable_id = $1")
            .bind(provider_id.as_uuid())
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all posts with a specific tag
    pub async fn find_posts_with_tag(tag_id: TagId, pool: &PgPool) -> Result<Vec<Uuid>> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT taggable_id FROM taggables WHERE tag_id = $1 AND taggable_type = 'post'",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    /// Find all providers with a specific tag
    pub async fn find_providers_with_tag(tag_id: TagId, pool: &PgPool) -> Result<Vec<Uuid>> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT taggable_id FROM taggables WHERE tag_id = $1 AND taggable_type = 'provider'",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }
}
