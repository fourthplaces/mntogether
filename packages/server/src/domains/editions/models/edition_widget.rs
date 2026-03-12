use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A non-post widget placed in an edition layout (section header, weather, hotline bar).
/// Widgets are independent layout items — peers of rows, not children.
/// Config is JSONB because each widget type has a genuinely different shape.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EditionWidget {
    pub id: Uuid,
    pub edition_row_id: Option<Uuid>,
    pub widget_type: String,
    pub slot_index: i32,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub edition_id: Uuid,
    pub sort_order: i32,
    pub section_id: Option<Uuid>,
}

impl EditionWidget {
    /// Create a new widget as an independent layout item in an edition.
    pub async fn create(
        edition_id: Uuid,
        widget_type: &str,
        sort_order: i32,
        section_id: Option<Uuid>,
        config: serde_json::Value,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO edition_widgets (edition_id, widget_type, sort_order, section_id, slot_index, config)
            VALUES ($1, $2, $3, $4, 0, $5)
            RETURNING *
            "#,
        )
        .bind(edition_id)
        .bind(widget_type)
        .bind(sort_order)
        .bind(section_id)
        .bind(&config)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update a widget's config.
    pub async fn update(
        id: Uuid,
        config: serde_json::Value,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_widgets
            SET config = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&config)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update a widget's sort_order (for reordering).
    pub async fn update_sort_order(
        id: Uuid,
        sort_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_widgets
            SET sort_order = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update a widget's section assignment.
    pub async fn update_section(
        id: Uuid,
        section_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_widgets
            SET section_id = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(section_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete a widget.
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM edition_widgets WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all widgets in an edition, ordered by sort_order.
    pub async fn find_by_edition(edition_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM edition_widgets WHERE edition_id = $1 ORDER BY sort_order ASC",
        )
        .bind(edition_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all widgets in a specific row, ordered by slot_index.
    /// Kept for backward compatibility during transition.
    pub async fn find_by_row(edition_row_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM edition_widgets WHERE edition_row_id = $1 ORDER BY slot_index ASC",
        )
        .bind(edition_row_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

}
