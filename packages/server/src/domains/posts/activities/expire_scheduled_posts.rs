//! Expire posts whose schedules have all passed.

use anyhow::Result;
use tracing::info;

use crate::domains::posts::models::Post;
use crate::kernel::ServerDeps;

/// Expire posts whose schedules have all passed.
/// Called by the Posts service handler on a daily schedule.
pub async fn expire_scheduled_posts(deps: &ServerDeps) -> Result<u64> {
    let expired_count = Post::expire_by_schedule(&deps.db_pool).await?;
    info!(expired_count, "Sweep: expired posts by schedule");
    Ok(expired_count)
}
