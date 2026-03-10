use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A topic section within an edition. Groups rows by topic for visual separation.
///
/// Sections are created by the layout engine (from Root Signal topic data) or
/// manually by editors. Rows with `section_id = NULL` render above the fold
/// (before any section divider). Deleting a section ungroups its rows
/// (sets their section_id to NULL via ON DELETE SET NULL).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EditionSection {
    pub id: Uuid,
    pub edition_id: Uuid,
    pub title: String,
    pub subtitle: Option<String>,
    pub topic_slug: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl EditionSection {
    /// Create a new section in an edition.
    pub async fn create(
        edition_id: Uuid,
        title: &str,
        subtitle: Option<&str>,
        topic_slug: Option<&str>,
        sort_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO edition_sections (edition_id, title, subtitle, topic_slug, sort_order)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(edition_id)
        .bind(title)
        .bind(subtitle)
        .bind(topic_slug)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all sections for an edition, ordered by sort_order.
    pub async fn find_by_edition(edition_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM edition_sections WHERE edition_id = $1 ORDER BY sort_order ASC",
        )
        .bind(edition_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a single section by ID.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM edition_sections WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Update a section's title, subtitle, or topic_slug.
    pub async fn update(
        id: Uuid,
        title: Option<&str>,
        subtitle: Option<Option<&str>>,
        topic_slug: Option<Option<&str>>,
        pool: &PgPool,
    ) -> Result<Self> {
        // Build update dynamically to handle nullable optional fields
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_sections
            SET title = COALESCE($2, title),
                subtitle = CASE WHEN $3::bool THEN $4 ELSE subtitle END,
                topic_slug = CASE WHEN $5::bool THEN $6 ELSE topic_slug END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(subtitle.is_some())
        .bind(subtitle.flatten())
        .bind(topic_slug.is_some())
        .bind(topic_slug.flatten())
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Reorder sections within an edition.
    pub async fn reorder(edition_id: Uuid, section_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        for (i, section_id) in section_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE edition_sections SET sort_order = $1 WHERE id = $2 AND edition_id = $3",
            )
            .bind(i as i32)
            .bind(section_id)
            .bind(edition_id)
            .execute(pool)
            .await?;
        }

        Self::find_by_edition(edition_id, pool).await
    }

    /// Delete a section. Rows assigned to it will have section_id set to NULL
    /// (via ON DELETE SET NULL in the FK constraint).
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM edition_sections WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
