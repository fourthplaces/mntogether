use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A non-post widget placed in an edition row (section header, weather, hotline bar).
/// Config is JSONB because each widget type has a genuinely different shape.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EditionWidget {
    pub id: Uuid,
    pub edition_row_id: Uuid,
    pub widget_type: String,
    pub slot_index: i32,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl EditionWidget {
    /// Create a new widget in a row.
    pub async fn create(
        edition_row_id: Uuid,
        widget_type: &str,
        slot_index: i32,
        config: serde_json::Value,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO edition_widgets (edition_row_id, widget_type, slot_index, config)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(edition_row_id)
        .bind(widget_type)
        .bind(slot_index)
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

    /// Delete a widget.
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM edition_widgets WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all widgets in a specific row, ordered by slot_index.
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
