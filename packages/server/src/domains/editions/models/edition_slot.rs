use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A slot within an edition row — can hold either a post or a widget.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EditionSlot {
    pub id: Uuid,
    pub edition_row_id: Uuid,
    pub kind: String, // 'post' or 'widget'
    pub post_id: Option<Uuid>,
    pub widget_id: Option<Uuid>,
    pub post_template: Option<String>,
    pub widget_template: Option<String>,
    pub slot_index: i32,
    pub created_at: DateTime<Utc>,
}

/// A post slot with its post data pre-loaded (avoids N+1 queries).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SlotWithPost {
    pub id: Uuid,
    pub edition_row_id: Uuid,
    pub kind: String,
    pub post_id: Option<Uuid>,
    pub widget_id: Option<Uuid>,
    pub post_template: Option<String>,
    pub widget_template: Option<String>,
    pub slot_index: i32,
    pub created_at: DateTime<Utc>,
    // Post fields (joined from posts table, nullable for widget slots)
    pub post_title: Option<String>,
    pub post_post_type: Option<String>,
    pub post_weight: Option<String>,
    pub post_status: Option<String>,
    // Widget fields (joined from widgets table, nullable for post slots)
    pub widget_type: Option<String>,
    pub widget_authoring_mode: Option<String>,
    pub widget_data: Option<serde_json::Value>,
}

impl EditionSlot {
    /// Create a new post slot.
    pub async fn create(
        edition_row_id: Uuid,
        post_id: Uuid,
        post_template: &str,
        slot_index: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO edition_slots (edition_row_id, kind, post_id, post_template, slot_index)
            VALUES ($1, 'post', $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(edition_row_id)
        .bind(post_id)
        .bind(post_template)
        .bind(slot_index)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new widget slot.
    pub async fn create_widget_slot(
        edition_row_id: Uuid,
        widget_id: Uuid,
        slot_index: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO edition_slots (edition_row_id, kind, widget_id, slot_index)
            VALUES ($1, 'widget', $2, $3)
            RETURNING *
            "#,
        )
        .bind(edition_row_id)
        .bind(widget_id)
        .bind(slot_index)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all slots in a specific row, ordered by slot_index.
    pub async fn find_by_row(edition_row_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM edition_slots WHERE edition_row_id = $1 ORDER BY slot_index ASC",
        )
        .bind(edition_row_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all slots across all rows of an edition (via JOIN).
    pub async fn find_by_edition(edition_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT es.*
            FROM edition_slots es
            INNER JOIN edition_rows er ON es.edition_row_id = er.id
            WHERE er.edition_id = $1
            ORDER BY er.sort_order ASC, es.slot_index ASC
            "#,
        )
        .bind(edition_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Move a slot to a different row / position.
    pub async fn move_to(
        id: Uuid,
        target_row_id: Uuid,
        slot_index: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_slots
            SET edition_row_id = $2, slot_index = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(target_row_id)
        .bind(slot_index)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Change the post template (visual treatment) for a post slot.
    pub async fn change_template(id: Uuid, post_template: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE edition_slots
            SET post_template = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(post_template)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete a slot (remove a post or widget from the edition).
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM edition_slots WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all slots in a row, with post and widget data joined.
    pub async fn find_by_row_with_content(
        edition_row_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<SlotWithPost>> {
        sqlx::query_as::<_, SlotWithPost>(
            r#"
            SELECT
                es.id, es.edition_row_id, es.kind, es.post_id, es.widget_id,
                es.post_template, es.widget_template, es.slot_index, es.created_at,
                p.title AS post_title,
                p.post_type AS post_post_type,
                p.weight AS post_weight,
                p.status AS post_status,
                w.widget_type AS widget_type,
                w.authoring_mode AS widget_authoring_mode,
                w.data AS widget_data
            FROM edition_slots es
            LEFT JOIN posts p ON p.id = es.post_id
            LEFT JOIN widgets w ON w.id = es.widget_id
            WHERE es.edition_row_id = $1
            ORDER BY es.slot_index ASC
            "#,
        )
        .bind(edition_row_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Replace all slots for a row (used by layout engine).
    /// Takes post slots as (post_id, post_template, slot_index) tuples.
    pub async fn replace_for_row(
        edition_row_id: Uuid,
        slots: &[(Uuid, String, i32)], // (post_id, post_template, slot_index)
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query("DELETE FROM edition_slots WHERE edition_row_id = $1")
            .bind(edition_row_id)
            .execute(pool)
            .await?;

        let mut results = Vec::with_capacity(slots.len());
        for (post_id, post_template, slot_index) in slots {
            let slot =
                Self::create(edition_row_id, *post_id, post_template, *slot_index, pool).await?;
            results.push(slot);
        }
        Ok(results)
    }
}
