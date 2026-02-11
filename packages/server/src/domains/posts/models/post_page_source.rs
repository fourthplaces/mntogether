use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// Link between a post and the page snapshot it was extracted from
pub struct PostPageSource;

impl PostPageSource {
    /// Link a post to its source page snapshot (idempotent)
    pub async fn link(post_id: PostId, page_snapshot_id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "INSERT INTO post_page_sources (post_id, page_snapshot_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(post_id.into_uuid())
        .bind(page_snapshot_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
