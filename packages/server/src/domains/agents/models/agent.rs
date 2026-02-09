use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::MemberId;

/// An autonomous entity with a member identity and a role.
///
/// Roles:
/// - `assistant`: responds to users in chat
/// - `curator`: discovers websites, extracts posts, enriches, monitors
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Agent {
    pub id: Uuid,
    pub member_id: Uuid,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    pub fn member_id(&self) -> MemberId {
        MemberId::from(self.member_id)
    }

    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM agents ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM agents WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_role(role: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM agents WHERE role = $1 ORDER BY created_at DESC")
            .bind(role)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agents WHERE status = 'active' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_active_curators(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agents WHERE role = 'curator' AND status = 'active' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new agent with a synthetic member identity.
    ///
    /// Provisions a member row with `expo_push_token = "agent:{slug}"` to prevent
    /// collision with real users. The slug is derived from the display_name.
    pub async fn create(
        display_name: &str,
        role: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let slug = display_name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .trim_matches('-')
            .to_string();
        let push_token = format!("agent:{}", slug);

        let member_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO members (expo_push_token, searchable_text, active, notification_count_this_week)
            VALUES ($1, $2, true, 0)
            ON CONFLICT (expo_push_token) DO UPDATE SET searchable_text = EXCLUDED.searchable_text
            RETURNING id
            "#,
        )
        .bind(&push_token)
        .bind(display_name)
        .fetch_one(pool)
        .await?;

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO agents (member_id, display_name, role, status)
            VALUES ($1, $2, $3, 'draft')
            RETURNING *
            "#,
        )
        .bind(member_id)
        .bind(display_name)
        .bind(role)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_display_name(id: Uuid, display_name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agents SET display_name = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(display_name)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn set_status(id: Uuid, status: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agents SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM agents WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
