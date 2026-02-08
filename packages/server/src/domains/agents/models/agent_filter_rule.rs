use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentFilterRule {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub rule_text: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl AgentFilterRule {
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_filter_rules WHERE agent_id = $1 ORDER BY sort_order, created_at",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_active_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_filter_rules WHERE agent_id = $1 AND is_active = true ORDER BY sort_order, created_at",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(agent_id: Uuid, rule_text: &str, sort_order: i32, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO agent_filter_rules (agent_id, rule_text, sort_order) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(agent_id)
        .bind(rule_text)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(id: Uuid, rule_text: &str, sort_order: i32, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agent_filter_rules SET rule_text = $2, sort_order = $3 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(rule_text)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM agent_filter_rules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
