use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentSearchQuery {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub query_text: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl AgentSearchQuery {
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_search_queries WHERE agent_id = $1 ORDER BY sort_order, created_at",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_active_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_search_queries WHERE agent_id = $1 AND is_active = true ORDER BY sort_order, created_at",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(agent_id: Uuid, query_text: &str, sort_order: i32, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO agent_search_queries (agent_id, query_text, sort_order) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(agent_id)
        .bind(query_text)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(id: Uuid, query_text: &str, sort_order: i32, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agent_search_queries SET query_text = $2, sort_order = $3 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(query_text)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn toggle_active(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agent_search_queries SET is_active = NOT is_active WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM agent_search_queries WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
