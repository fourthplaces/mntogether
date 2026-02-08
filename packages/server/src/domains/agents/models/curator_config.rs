use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Curator-specific configuration (purpose, audience, schedules).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentCuratorConfig {
    pub agent_id: Uuid,
    pub purpose: String,
    pub audience_roles: Vec<String>,
    pub schedule_discover: Option<String>,
    pub schedule_monitor: Option<String>,
}

impl AgentCuratorConfig {
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_curator_configs WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(agent_id: Uuid, purpose: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO agent_curator_configs (agent_id, purpose) VALUES ($1, $2) RETURNING *",
        )
        .bind(agent_id)
        .bind(purpose)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(
        agent_id: Uuid,
        purpose: &str,
        audience_roles: &[String],
        schedule_discover: Option<&str>,
        schedule_monitor: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE agent_curator_configs
            SET purpose = $2, audience_roles = $3, schedule_discover = $4, schedule_monitor = $5
            WHERE agent_id = $1
            RETURNING *
            "#,
        )
        .bind(agent_id)
        .bind(purpose)
        .bind(audience_roles)
        .bind(schedule_discover)
        .bind(schedule_monitor)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
