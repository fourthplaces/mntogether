use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Assistant-specific configuration (preamble for chat, config name).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentAssistantConfig {
    pub agent_id: Uuid,
    pub preamble: String,
    pub config_name: String,
}

impl AgentAssistantConfig {
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_assistant_configs WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_config_name(config_name: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_assistant_configs WHERE config_name = $1",
        )
        .bind(config_name)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(agent_id: Uuid, preamble: &str, config_name: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO agent_assistant_configs (agent_id, preamble, config_name) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(agent_id)
        .bind(preamble)
        .bind(config_name)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(agent_id: Uuid, preamble: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agent_assistant_configs SET preamble = $2 WHERE agent_id = $1 RETURNING *",
        )
        .bind(agent_id)
        .bind(preamble)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}

pub const ADMIN_AGENT_PREAMBLE: &str = r#"You are an admin assistant for MN Together, a resource-sharing platform.
You can help administrators:
- Approve or reject listings
- Scrape websites for new resources
- Generate website assessments
- Search and filter listings
- Manage organizations

Be helpful and proactive. If an admin asks to do something, use the appropriate tool."#;

pub const PUBLIC_AGENT_PREAMBLE: &str = r#"You are MN Together Guide, a friendly community resource navigator for Minnesota.

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
