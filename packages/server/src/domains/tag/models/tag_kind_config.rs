use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::tag::Tag;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TagKindConfig {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl TagKindConfig {
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM tag_kinds ORDER BY display_name")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_slug(slug: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM tag_kinds WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM tag_kinds WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn create(
        slug: &str,
        display_name: &str,
        description: Option<&str>,
        allowed_resource_types: &[String],
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO tag_kinds (slug, display_name, description, allowed_resource_types)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(slug)
        .bind(display_name)
        .bind(description)
        .bind(allowed_resource_types)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(
        id: Uuid,
        display_name: &str,
        description: Option<&str>,
        allowed_resource_types: &[String],
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE tag_kinds
             SET display_name = $2, description = $3, allowed_resource_types = $4
             WHERE id = $1
             RETURNING *",
        )
        .bind(id)
        .bind(display_name)
        .bind(description)
        .bind(allowed_resource_types)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM tag_kinds WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn tag_count_for_slug(slug: &str, pool: &PgPool) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags WHERE kind = $1")
            .bind(slug)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Find all tag kinds that allow a given resource type (e.g., "post").
    pub async fn find_for_resource_type(resource_type: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM tag_kinds WHERE $1 = ANY(allowed_resource_types) ORDER BY slug",
        )
        .bind(resource_type)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

/// Build dynamic tag instructions for AI extraction prompts.
///
/// Queries tag_kinds where 'post' is in allowed_resource_types,
/// then loads all tag values for each kind to construct prompt instructions.
pub async fn build_tag_instructions(pool: &PgPool) -> Result<String> {
    let kinds = TagKindConfig::find_for_resource_type("post", pool).await?;
    build_tag_instructions_from_kinds(&kinds, pool).await
}

/// Build tag instructions for specific tag kind IDs (e.g., agent's required kinds).
///
/// Only builds instructions for the given tag kind IDs. Returns empty string
/// if none of the IDs resolve or have tags.
pub async fn build_tag_instructions_for_kinds(
    tag_kind_ids: &[Uuid],
    pool: &PgPool,
) -> Result<String> {
    if tag_kind_ids.is_empty() {
        return Ok(String::new());
    }

    let mut kinds = Vec::new();
    for id in tag_kind_ids {
        if let Ok(kind) = TagKindConfig::find_by_id(*id, pool).await {
            kinds.push(kind);
        }
    }

    build_tag_instructions_from_kinds(&kinds, pool).await
}

/// Build tag instructions from a list of TagKindConfig entries.
async fn build_tag_instructions_from_kinds(
    kinds: &[TagKindConfig],
    pool: &PgPool,
) -> Result<String> {
    if kinds.is_empty() {
        return Ok(String::new());
    }

    let mut lines = Vec::new();

    for kind in kinds {
        // Skip audience_role since it's handled separately
        if kind.slug == "audience_role" {
            continue;
        }

        let tags = Tag::find_by_kind(&kind.slug, pool).await?;
        if tags.is_empty() {
            continue;
        }

        let values: Vec<&str> = tags.iter().map(|t| t.value.as_str()).collect();
        let description = kind
            .description
            .as_deref()
            .unwrap_or(&kind.display_name);

        lines.push(format!(
            "  - **{}**: {} — Array from: {}",
            kind.slug,
            description,
            values
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Ok(lines.join("\n"))
}
