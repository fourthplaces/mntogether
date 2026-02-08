use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentWebsite {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub website_id: Uuid,
    pub discovered_at: DateTime<Utc>,
}

impl AgentWebsite {
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_websites WHERE agent_id = $1 ORDER BY discovered_at DESC",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_website(website_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_websites WHERE website_id = $1 ORDER BY discovered_at DESC",
        )
        .bind(website_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Link a website to an agent. Upserts — does nothing if already linked.
    pub async fn link(agent_id: Uuid, website_id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO agent_websites (agent_id, website_id)
            VALUES ($1, $2)
            ON CONFLICT (agent_id, website_id) DO UPDATE SET discovered_at = agent_websites.discovered_at
            RETURNING *
            "#,
        )
        .bind(agent_id)
        .bind(website_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn unlink(agent_id: Uuid, website_id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM agent_websites WHERE agent_id = $1 AND website_id = $2")
            .bind(agent_id)
            .bind(website_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
