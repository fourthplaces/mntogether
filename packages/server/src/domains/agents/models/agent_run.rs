use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentRun {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub step: String,
    pub trigger_type: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentRun {
    pub async fn create(
        agent_id: Uuid,
        step: &str,
        trigger_type: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO agent_runs (agent_id, step, trigger_type) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(agent_id)
        .bind(step)
        .bind(trigger_type)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn complete(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agent_runs SET status = 'completed', completed_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn fail(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE agent_runs SET status = 'failed', completed_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM agent_runs WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_agent(agent_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_runs WHERE agent_id = $1 ORDER BY started_at DESC",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_recent(agent_id: Uuid, limit: i64, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_runs WHERE agent_id = $1 ORDER BY started_at DESC LIMIT $2",
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentRunStat {
    pub id: Uuid,
    pub run_id: Uuid,
    pub stat_key: String,
    pub stat_value: i32,
}

impl AgentRunStat {
    pub async fn create_batch(
        run_id: Uuid,
        stats: &[(&str, i32)],
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let mut results = Vec::new();
        for (key, value) in stats {
            let stat = sqlx::query_as::<_, Self>(
                "INSERT INTO agent_run_stats (run_id, stat_key, stat_value) VALUES ($1, $2, $3) RETURNING *",
            )
            .bind(run_id)
            .bind(key)
            .bind(value)
            .fetch_one(pool)
            .await?;
            results.push(stat);
        }
        Ok(results)
    }

    pub async fn find_by_run(run_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM agent_run_stats WHERE run_id = $1 ORDER BY stat_key",
        )
        .bind(run_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
