//! Organization-level post cleanup workflow
//!
//! Safety net for LLM extraction imperfections. Catches duplicates that
//! GPT-5 Mini missed during extraction by running a cleanup pass with GPT-5.
//!
//! Flow:
//! 1. Load org, validate exists
//! 2. Run dedup (single LLM call for all active + pending posts)
//! 3. Purge rejected posts (soft-delete)

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, OrganizationId};
use crate::domains::organization::models::Organization;
use crate::domains::posts::activities::deduplication::{
    purge_rejected_posts_for_org, stage_cross_source_dedup, StageCrossSourceResult,
};
use crate::impl_restate_serde;
use crate::kernel::{ServerDeps, GPT_5};

// =============================================================================
// Journaling wrapper types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PurgeResult {
    count: usize,
}

impl_restate_serde!(PurgeResult);

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanUpOrgPostsRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(CleanUpOrgPostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanUpOrgPostsResult {
    pub duplicates_found: i32,
    pub proposals_staged: i32,
    pub rejected_purged: i32,
    pub status: String,
}

impl_restate_serde!(CleanUpOrgPostsResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "CleanUpOrgPostsWorkflow"]
pub trait CleanUpOrgPostsWorkflow {
    async fn run(req: CleanUpOrgPostsRequest) -> Result<CleanUpOrgPostsResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct CleanUpOrgPostsWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl CleanUpOrgPostsWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl CleanUpOrgPostsWorkflow for CleanUpOrgPostsWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: CleanUpOrgPostsRequest,
    ) -> Result<CleanUpOrgPostsResult, HandlerError> {
        let org_id = OrganizationId::from(req.organization_id);
        info!(organization_id = %req.organization_id, "Starting clean up org posts workflow");

        let pool = &self.deps.db_pool;

        // Validate org exists
        let _org = Organization::find_by_id(org_id, pool)
            .await
            .map_err(|e| TerminalError::new(format!("Organization not found: {}", e)))?;

        // Phase 1: Deduplicate posts (single LLM call, one batch)
        ctx.set("status", "Deduplicating posts...".to_string());

        let dedup_result = match ctx
            .run(|| async {
                stage_cross_source_dedup(
                    req.organization_id,
                    GPT_5,
                    self.deps.ai.as_ref(),
                    pool,
                )
                .await
                .map_err(Into::into)
            })
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    organization_id = %req.organization_id,
                    error = %e,
                    "Dedup failed, continuing to purge"
                );
                StageCrossSourceResult {
                    batch_id: None,
                    proposals_staged: 0,
                }
            }
        };

        info!(
            organization_id = %req.organization_id,
            proposals_staged = dedup_result.proposals_staged,
            "Dedup phase complete"
        );

        // Phase 2: Purge rejected posts (direct soft-delete)
        ctx.set("status", "Purging rejected posts...".to_string());

        let purged = match ctx
            .run(|| async {
                purge_rejected_posts_for_org(req.organization_id, pool)
                    .await
                    .map(|count| PurgeResult { count })
                    .map_err(Into::into)
            })
            .await
        {
            Ok(r) => r.count,
            Err(e) => {
                warn!(
                    organization_id = %req.organization_id,
                    error = %e,
                    "Purge failed"
                );
                0
            }
        };

        info!(
            organization_id = %req.organization_id,
            rejected_purged = purged,
            "Purge phase complete"
        );

        ctx.set("status", "Completed".to_string());

        Ok(CleanUpOrgPostsResult {
            duplicates_found: dedup_result.proposals_staged as i32,
            proposals_staged: dedup_result.proposals_staged as i32,
            rejected_purged: purged as i32,
            status: "completed".to_string(),
        })
    }

    async fn get_status(
        &self,
        ctx: SharedWorkflowContext<'_>,
        _req: EmptyRequest,
    ) -> Result<String, HandlerError> {
        Ok(ctx
            .get::<String>("status")
            .await?
            .unwrap_or_else(|| "pending".to_string()))
    }
}
