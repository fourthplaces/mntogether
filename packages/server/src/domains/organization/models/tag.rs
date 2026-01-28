use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Universal tag - can be associated with any entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub kind: String,  // 'service', 'language', 'community', etc.
    pub value: String, // 'food_assistance', 'spanish', etc.
    pub created_at: DateTime<Utc>,
}

/// Junction table: organization <-> tag
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TagOnOrganization {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub tag_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Tag Queries
// =============================================================================

impl Tag {
    /// Find tag by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        let tag = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(tag)
    }

    /// Find or create tag by kind and value
    pub async fn find_or_create(kind: &str, value: &str, pool: &PgPool) -> Result<Self> {
        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (kind, value)
            VALUES ($1, $2)
            ON CONFLICT (kind, value) DO UPDATE SET kind = EXCLUDED.kind
            RETURNING *
            "#,
        )
        .bind(kind)
        .bind(value)
        .fetch_one(pool)
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

    /// Find all tags for an organization
    pub async fn find_for_organization(organization_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.*
            FROM tags t
            INNER JOIN tags_on_organizations tao ON tao.tag_id = t.id
            WHERE tao.organization_id = $1
            ORDER BY t.kind, t.value
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;
        Ok(tags)
    }
}

// =============================================================================
// TagOnOrganization Queries
// =============================================================================

impl TagOnOrganization {
    /// Associate a tag with an organization
    pub async fn create(organization_id: Uuid, tag_id: Uuid, pool: &PgPool) -> Result<Self> {
        let association = sqlx::query_as::<_, TagOnOrganization>(
            r#"
            INSERT INTO tags_on_organizations (organization_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT (organization_id, tag_id) DO UPDATE
            SET organization_id = EXCLUDED.organization_id
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(tag_id)
        .fetch_one(pool)
        .await?;
        Ok(association)
    }

    /// Remove a tag from an organization
    pub async fn delete(organization_id: Uuid, tag_id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM tags_on_organizations WHERE organization_id = $1 AND tag_id = $2")
            .bind(organization_id)
            .bind(tag_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Remove all tags from an organization
    pub async fn delete_all_for_organization(organization_id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM tags_on_organizations WHERE organization_id = $1")
            .bind(organization_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all organizations with a specific tag
    pub async fn find_organizations_with_tag(tag_id: Uuid, pool: &PgPool) -> Result<Vec<Uuid>> {
        let org_ids: Vec<(Uuid,)> =
            sqlx::query_as("SELECT organization_id FROM tags_on_organizations WHERE tag_id = $1")
                .bind(tag_id)
                .fetch_all(pool)
                .await?;
        Ok(org_ids.into_iter().map(|(id,)| id).collect())
    }
}
