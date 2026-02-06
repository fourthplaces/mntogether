use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::MemberId;

/// An AI agent with a real member identity for message authorship.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Agent {
    pub id: Uuid,
    pub member_id: Uuid,
    pub display_name: String,
    pub preamble: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    /// Get the default active agent, creating one if none exists.
    pub async fn get_or_create_default(pool: &PgPool) -> Result<Self> {
        if let Some(agent) = Self::find_first_active(pool).await? {
            return Ok(agent);
        }
        Self::create_default(pool).await
    }

    /// Typed member ID for use as message author.
    pub fn member_id(&self) -> MemberId {
        MemberId::from(self.member_id)
    }

    async fn find_first_active(pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agents WHERE is_active = true ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    async fn create_default(pool: &PgPool) -> Result<Self> {
        // Create a synthetic member for the agent
        let member_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO members (expo_push_token, searchable_text, active, notification_count_this_week)
            VALUES ('agent:default', 'AI Admin Assistant', true, 0)
            ON CONFLICT (expo_push_token) DO UPDATE SET searchable_text = EXCLUDED.searchable_text
            RETURNING id
            "#,
        )
        .fetch_one(pool)
        .await?;

        let preamble = r#"You are an admin assistant for MN Together, a resource-sharing platform.
You can help administrators:
- Approve or reject listings
- Scrape websites for new resources
- Generate website assessments
- Search and filter listings
- Manage organizations

Be helpful and proactive. If an admin asks to do something, use the appropriate tool."#;

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO agents (member_id, display_name, preamble)
            VALUES ($1, 'MN Together Assistant', $2)
            ON CONFLICT (member_id) DO UPDATE SET is_active = true
            RETURNING *
            "#,
        )
        .bind(member_id)
        .bind(preamble)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
