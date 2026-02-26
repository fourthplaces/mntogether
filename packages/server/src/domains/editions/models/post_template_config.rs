use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A post visual treatment template (e.g. "feature", "gazette", "ticker").
/// Defines which post types it can render and character limits for truncation.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostTemplateConfig {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub compatible_types: Vec<String>,
    pub body_target: i32,
    pub body_max: i32,
    pub title_max: i32,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl PostTemplateConfig {
    /// Load all post templates, ordered by sort_order.
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM post_template_configs ORDER BY sort_order ASC")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Find a post template by slug.
    pub async fn find_by_slug(slug: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM post_template_configs WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find all post templates compatible with a given post type.
    pub async fn find_compatible(post_type: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_template_configs WHERE $1 = ANY(compatible_types) ORDER BY sort_order ASC",
        )
        .bind(post_type)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Check if this template is compatible with a given post type.
    pub fn is_compatible(&self, post_type: &str) -> bool {
        self.compatible_types.iter().any(|t| t == post_type)
    }
}
