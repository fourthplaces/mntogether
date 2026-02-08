use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentRequiredTagKind {
    pub agent_id: Uuid,
    pub tag_kind_id: Uuid,
}

impl AgentRequiredTagKind {
    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_required_tag_kinds WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Replace all required tag kinds for an agent with the given set.
    pub async fn set_for_agent(
        agent_id: Uuid,
        tag_kind_ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query("DELETE FROM agent_required_tag_kinds WHERE agent_id = $1")
            .bind(agent_id)
            .execute(pool)
            .await?;

        let mut results = Vec::new();
        for tag_kind_id in tag_kind_ids {
            let row = sqlx::query_as::<_, Self>(
                "INSERT INTO agent_required_tag_kinds (agent_id, tag_kind_id) VALUES ($1, $2) RETURNING *",
            )
            .bind(agent_id)
            .bind(tag_kind_id)
            .fetch_one(pool)
            .await?;
            results.push(row);
        }
        Ok(results)
    }

    pub async fn remove(agent_id: Uuid, tag_kind_id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "DELETE FROM agent_required_tag_kinds WHERE agent_id = $1 AND tag_kind_id = $2",
        )
        .bind(agent_id)
        .bind(tag_kind_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
