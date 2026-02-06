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
    pub config_name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    /// Get the default active agent, creating one if none exists.
    pub async fn get_or_create_default(pool: &PgPool) -> Result<Self> {
        Self::get_or_create_by_config("admin", pool).await
    }

    /// Get or create an agent by config name.
    pub async fn get_or_create_by_config(config: &str, pool: &PgPool) -> Result<Self> {
        if let Some(agent) = Self::find_by_config(config, pool).await? {
            return Ok(agent);
        }
        Self::create_for_config(config, pool).await
    }

    /// Typed member ID for use as message author.
    pub fn member_id(&self) -> MemberId {
        MemberId::from(self.member_id)
    }

    async fn find_by_config(config: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agents WHERE config_name = $1 AND is_active = true LIMIT 1",
        )
        .bind(config)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    async fn create_for_config(config: &str, pool: &PgPool) -> Result<Self> {
        let (push_token, searchable, display_name, preamble) = match config {
            "public" => (
                "agent:public",
                "MN Together Guide",
                "MN Together Guide",
                PUBLIC_AGENT_PREAMBLE,
            ),
            _ => (
                "agent:default",
                "AI Admin Assistant",
                "MN Together Assistant",
                ADMIN_AGENT_PREAMBLE,
            ),
        };

        // Create a synthetic member for the agent
        let member_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO members (expo_push_token, searchable_text, active, notification_count_this_week)
            VALUES ($1, $2, true, 0)
            ON CONFLICT (expo_push_token) DO UPDATE SET searchable_text = EXCLUDED.searchable_text
            RETURNING id
            "#,
        )
        .bind(push_token)
        .bind(searchable)
        .fetch_one(pool)
        .await?;

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO agents (member_id, display_name, preamble, config_name)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (member_id) DO UPDATE SET is_active = true, config_name = EXCLUDED.config_name
            RETURNING *
            "#,
        )
        .bind(member_id)
        .bind(display_name)
        .bind(preamble)
        .bind(config)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}

const ADMIN_AGENT_PREAMBLE: &str = r#"You are an admin assistant for MN Together, a resource-sharing platform.
You can help administrators:
- Approve or reject listings
- Scrape websites for new resources
- Generate website assessments
- Search and filter listings
- Manage organizations

Be helpful and proactive. If an admin asks to do something, use the appropriate tool."#;

const PUBLIC_AGENT_PREAMBLE: &str = r#"You are MN Together Guide, a friendly community resource navigator for Minnesota.

Help people find:
- Social services (food, housing, healthcare, legal aid)
- Volunteer and civic engagement opportunities
- Local businesses that give back to the community
- Support for specific populations (seniors, refugees, youth, etc.)

Rules:
- Always use your search_posts tool before answering. Do not guess.
- Present results as a brief summary: what you found and why it's relevant.
- Keep responses concise (2-4 sentences). The structured results are shown separately in the UI.
- If no results found: acknowledge it, suggest broadening the search, recommend calling 211.
- Be warm, respectful, and concise. Many users may be in difficult situations.
- Never ask for personal information.
- For emergencies, remind them to call 911."#;
