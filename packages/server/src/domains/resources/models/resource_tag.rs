//! ResourceTag model - direct tag associations for resources
//!
//! This provides a direct relationship table for resource tags, separate from
//! the polymorphic taggables table. This allows for more efficient queries
//! and clearer relationships.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ResourceId, TagId};
use crate::domains::tag::models::Tag;

/// ResourceTag - direct association between a resource and a tag
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResourceTag {
    pub id: Uuid,
    pub resource_id: ResourceId,
    pub tag_id: TagId,
    pub created_at: DateTime<Utc>,
}

impl ResourceTag {
    /// Find all tag associations for a resource
    pub async fn find_by_resource_id(resource_id: ResourceId, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Self>(
            "SELECT * FROM resource_tags WHERE resource_id = $1 ORDER BY created_at ASC",
        )
        .bind(resource_id)
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all tags for a resource (returns full Tag objects)
    pub async fn find_tags_for_resource(
        resource_id: ResourceId,
        pool: &PgPool,
    ) -> Result<Vec<Tag>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN resource_tags rt ON rt.tag_id = t.id
            WHERE rt.resource_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(resource_id)
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }

    /// Find all resources with a specific tag
    pub async fn find_resources_with_tag(tag_id: TagId, pool: &PgPool) -> Result<Vec<ResourceId>> {
        let ids = sqlx::query_scalar::<_, ResourceId>(
            "SELECT resource_id FROM resource_tags WHERE tag_id = $1",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await?;
        Ok(ids)
    }

    /// Check if a resource has a specific tag
    pub async fn exists(resource_id: ResourceId, tag_id: TagId, pool: &PgPool) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM resource_tags WHERE resource_id = $1 AND tag_id = $2)",
        )
        .bind(resource_id)
        .bind(tag_id)
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }

    /// Add a tag to a resource (upsert - ignores duplicates)
    pub async fn add(resource_id: ResourceId, tag_id: TagId, pool: &PgPool) -> Result<Self> {
        let tag = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO resource_tags (resource_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT (resource_id, tag_id) DO UPDATE
            SET resource_id = EXCLUDED.resource_id
            RETURNING *
            "#,
        )
        .bind(resource_id)
        .bind(tag_id)
        .fetch_one(pool)
        .await?;
        Ok(tag)
    }

    /// Remove a tag from a resource
    pub async fn remove(resource_id: ResourceId, tag_id: TagId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM resource_tags WHERE resource_id = $1 AND tag_id = $2")
            .bind(resource_id)
            .bind(tag_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Remove all tags from a resource
    pub async fn remove_all(resource_id: ResourceId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM resource_tags WHERE resource_id = $1")
            .bind(resource_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Add or find a tag by kind/value and associate with resource
    pub async fn add_tag_by_value(
        resource_id: ResourceId,
        kind: &str,
        value: &str,
        display_name: Option<String>,
        pool: &PgPool,
    ) -> Result<Tag> {
        // Find or create the tag
        let tag = Tag::find_or_create(kind, value, display_name, pool).await?;

        // Associate with resource
        Self::add(resource_id, tag.id, pool).await?;

        Ok(tag)
    }
}
