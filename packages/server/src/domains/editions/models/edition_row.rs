use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// An ordered row within an edition, referencing a row template.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EditionRow {
    pub id: Uuid,
    pub edition_id: Uuid,
    pub row_template_config_id: Uuid,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl EditionRow {
    /// Create a new row in an edition.
    pub async fn create(
        edition_id: Uuid,
        row_template_config_id: Uuid,
        sort_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO edition_rows (edition_id, row_template_config_id, sort_order)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(edition_id)
        .bind(row_template_config_id)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all rows for an edition, ordered by sort_order.
    pub async fn find_by_edition(edition_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM edition_rows WHERE edition_id = $1 ORDER BY sort_order ASC",
        )
        .bind(edition_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a single row by ID.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM edition_rows WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Update a row's template and/or sort order.
    pub async fn update(
        id: Uuid,
        row_template_config_id: Option<Uuid>,
        sort_order: Option<i32>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_rows
            SET row_template_config_id = COALESCE($2, row_template_config_id),
                sort_order = COALESCE($3, sort_order)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(row_template_config_id)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Reorder rows within an edition. Takes the row IDs in their new order.
    pub async fn reorder(edition_id: Uuid, row_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        for (i, row_id) in row_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE edition_rows SET sort_order = $1 WHERE id = $2 AND edition_id = $3",
            )
            .bind(i as i32)
            .bind(row_id)
            .bind(edition_id)
            .execute(pool)
            .await?;
        }

        Self::find_by_edition(edition_id, pool).await
    }

    /// Delete a row (cascades to its slots).
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM edition_rows WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
