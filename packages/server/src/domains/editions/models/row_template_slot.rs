use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// A slot definition within a row template. Defines weight constraint, post count,
/// optional type filter, and the default post template for this slot.
///
/// `post_template_slug` is the "good default" — the layout engine uses it when
/// filling slots. Editors can override the template per-slot in the admin UI
/// without changing the underlying recipe.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RowTemplateSlot {
    pub id: Uuid,
    pub row_template_config_id: Uuid,
    pub slot_index: i32,
    pub weight: String,
    pub count: i32,
    pub accepts: Option<Vec<String>>,
    pub post_template_slug: Option<String>,
}

impl RowTemplateSlot {
    /// Find all slot definitions for a specific row template, ordered by slot_index.
    pub async fn find_by_template(template_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM row_template_slots WHERE row_template_config_id = $1 ORDER BY slot_index ASC",
        )
        .bind(template_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Load all slot definitions across all templates (for bulk loading).
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM row_template_slots ORDER BY row_template_config_id, slot_index ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Check if this slot accepts a given post type.
    /// Returns true if `accepts` is None (any type) or contains the type.
    pub fn accepts_type(&self, post_type: &str) -> bool {
        match &self.accepts {
            None => true,
            Some(types) => types.iter().any(|t| t == post_type),
        }
    }

    /// Total number of posts this slot group can hold.
    pub fn capacity(&self) -> i32 {
        self.count
    }
}
