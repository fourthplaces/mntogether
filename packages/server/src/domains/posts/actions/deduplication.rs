//! Post deduplication actions - entry-point functions for deduplication
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle auth checks,
//! and return results directly.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{AppState, JobId, MemberId};
use crate::domains::posts::effects::deduplication::{apply_dedup_results, deduplicate_posts_llm};
use crate::domains::posts::events::PostEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Result of deduplication operation
#[derive(Debug, Clone)]
pub struct DeduplicationResult {
    pub job_id: Uuid,
    pub duplicates_found: i32,
    pub posts_merged: i32,
    pub posts_deleted: i32,
}

/// Deduplicate posts using LLM-based similarity (admin only)
/// Returns deduplication results directly.
pub async fn deduplicate_posts(
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<DeduplicationResult> {
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(ctx.deps())
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    info!(job_id = %job_id, "Starting LLM-based post deduplication");

    let websites = match Website::find_approved(&ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, "Failed to fetch websites for deduplication");
            ctx.emit(PostEvent::PostsDeduplicated {
                job_id,
                duplicates_found: 0,
                posts_merged: 0,
                posts_deleted: 0,
            });
            return Ok(DeduplicationResult {
                job_id: job_id.into_uuid(),
                duplicates_found: 0,
                posts_merged: 0,
                posts_deleted: 0,
            });
        }
    };

    let mut total_deleted = 0;
    let mut total_groups = 0;

    for website in &websites {
        let dedup_result =
            match deduplicate_posts_llm(website.id, ctx.deps().ai.as_ref(), &ctx.deps().db_pool)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(website_id = %website.id, error = %e, "Failed LLM deduplication");
                    continue;
                }
            };

        total_groups += dedup_result.duplicate_groups.len();

        let deleted =
            match apply_dedup_results(dedup_result, ctx.deps().ai.as_ref(), &ctx.deps().db_pool)
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    warn!(website_id = %website.id, error = %e, "Failed to apply deduplication");
                    continue;
                }
            };

        total_deleted += deleted;

        if deleted > 0 {
            info!(website_id = %website.id, deleted = deleted, "Deduplicated posts");
        }
    }

    info!(
        job_id = %job_id,
        total_groups = total_groups,
        total_deleted = total_deleted,
        "LLM deduplication complete"
    );

    ctx.emit(PostEvent::PostsDeduplicated {
        job_id,
        duplicates_found: total_groups,
        posts_merged: total_groups,
        posts_deleted: total_deleted,
    });

    Ok(DeduplicationResult {
        job_id: job_id.into_uuid(),
        duplicates_found: total_groups as i32,
        posts_merged: total_groups as i32,
        posts_deleted: total_deleted as i32,
    })
}
