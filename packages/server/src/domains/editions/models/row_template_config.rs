use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::row_template_slot::RowTemplateSlot;

/// A row layout template for the broadsheet (e.g. "hero-with-sidebar", "three-column").
/// Each template defines how many slots of which weight it contains.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RowTemplateConfig {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// A row template with its slot definitions pre-loaded.
#[derive(Debug, Clone)]
pub struct RowTemplateWithSlots {
    pub config: RowTemplateConfig,
    pub slots: Vec<RowTemplateSlot>,
}

impl RowTemplateConfig {
    /// Load all row templates, ordered by sort_order.
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM row_template_configs ORDER BY sort_order ASC")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find a row template by slug.
    pub async fn find_by_slug(slug: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM row_template_configs WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find a row template by id.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM row_template_configs WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Load all row templates with their slot definitions.
    pub async fn find_all_with_slots(pool: &PgPool) -> Result<Vec<RowTemplateWithSlots>> {
        let configs = Self::find_all(pool).await?;
        let all_slots = RowTemplateSlot::find_all(pool).await?;

        let results = configs
            .into_iter()
            .map(|config| {
                let slots: Vec<RowTemplateSlot> = all_slots
                    .iter()
                    .filter(|s| s.row_template_config_id == config.id)
                    .cloned()
                    .collect();
                RowTemplateWithSlots { config, slots }
            })
            .collect();

        Ok(results)
    }
}
