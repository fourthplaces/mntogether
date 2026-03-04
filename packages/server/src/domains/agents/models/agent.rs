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

    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM agents WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_member_id(member_id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM agents WHERE member_id = $1")
            .bind(member_id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

}
