//! Sync service (stateless)
//!
//! Batch/proposal management for data synchronization.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{SyncBatchId, SyncProposalId};
use crate::domains::posts::activities::post_sync_handler::PostProposalHandler;
use crate::domains::sync::activities::proposal_actions;
use crate::domains::sync::{SyncBatch, SyncProposal};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBatchesRequest {
    pub status: Option<String>,
    pub limit: Option<i32>,
}

impl_restate_serde!(ListBatchesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBatchRequest {
    pub id: Uuid,
}

impl_restate_serde!(GetBatchRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProposalsRequest {
    pub batch_id: Uuid,
}

impl_restate_serde!(ListProposalsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEntityProposalsRequest {
    pub entity_id: Uuid,
}

impl_restate_serde!(ListEntityProposalsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveProposalRequest {
    pub proposal_id: Uuid,
}

impl_restate_serde!(ApproveProposalRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectProposalRequest {
    pub proposal_id: Uuid,
}

impl_restate_serde!(RejectProposalRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveBatchRequest {
    pub batch_id: Uuid,
}

impl_restate_serde!(ApproveBatchRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectBatchRequest {
    pub batch_id: Uuid,
}

impl_restate_serde!(RejectBatchRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub id: Uuid,
    pub resource_type: String,
    pub status: String,
    pub proposal_count: i32,
}

impl_restate_serde!(BatchResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchListResult {
    pub batches: Vec<BatchResult>,
}

impl_restate_serde!(BatchListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalResult {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub operation: String,
    pub status: String,
}

impl_restate_serde!(ProposalResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalListResult {
    pub proposals: Vec<ProposalResult>,
}

impl_restate_serde!(ProposalListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityProposalResult {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub operation: String,
    pub status: String,
    pub entity_type: String,
    pub draft_entity_id: Option<Uuid>,
    pub target_entity_id: Option<Uuid>,
    pub reason: Option<String>,
    pub created_at: String,
}

impl_restate_serde!(EntityProposalResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityProposalListResult {
    pub proposals: Vec<EntityProposalResult>,
}

impl_restate_serde!(EntityProposalListResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Sync"]
pub trait SyncService {
    async fn list_batches(req: ListBatchesRequest) -> Result<BatchListResult, HandlerError>;
    async fn get_batch(req: GetBatchRequest) -> Result<BatchResult, HandlerError>;
    async fn list_proposals(
        req: ListProposalsRequest,
    ) -> Result<ProposalListResult, HandlerError>;
    async fn list_entity_proposals(
        req: ListEntityProposalsRequest,
    ) -> Result<EntityProposalListResult, HandlerError>;
    async fn approve_proposal(req: ApproveProposalRequest) -> Result<ProposalResult, HandlerError>;
    async fn reject_proposal(req: RejectProposalRequest) -> Result<ProposalResult, HandlerError>;
    async fn approve_batch(req: ApproveBatchRequest) -> Result<BatchResult, HandlerError>;
    async fn reject_batch(req: RejectBatchRequest) -> Result<BatchResult, HandlerError>;
}

pub struct SyncServiceImpl {
    deps: Arc<ServerDeps>,
}

impl SyncServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl SyncService for SyncServiceImpl {
    async fn list_batches(
        &self,
        ctx: Context<'_>,
        req: ListBatchesRequest,
    ) -> Result<BatchListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let limit = req.limit.unwrap_or(20);
        let batches = match req.status.as_deref() {
            Some("pending") => SyncBatch::find_pending(&self.deps.db_pool).await,
            _ => SyncBatch::find_recent(limit, &self.deps.db_pool).await,
        }
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(BatchListResult {
            batches: batches
                .into_iter()
                .map(|b| BatchResult {
                    id: b.id.into_uuid(),
                    resource_type: b.resource_type,
                    status: b.status,
                    proposal_count: b.proposal_count,
                })
                .collect(),
        })
    }

    async fn get_batch(
        &self,
        ctx: Context<'_>,
        req: GetBatchRequest,
    ) -> Result<BatchResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let batch = SyncBatch::find_by_id(SyncBatchId::from_uuid(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Batch not found"))?;

        Ok(BatchResult {
            id: batch.id.into_uuid(),
            resource_type: batch.resource_type,
            status: batch.status,
            proposal_count: batch.proposal_count,
        })
    }

    async fn list_proposals(
        &self,
        ctx: Context<'_>,
        req: ListProposalsRequest,
    ) -> Result<ProposalListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let proposals =
            SyncProposal::find_by_batch(SyncBatchId::from_uuid(req.batch_id), &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ProposalListResult {
            proposals: proposals
                .into_iter()
                .map(|p| ProposalResult {
                    id: p.id.into_uuid(),
                    batch_id: p.batch_id.into_uuid(),
                    operation: p.operation,
                    status: p.status,
                })
                .collect(),
        })
    }

    async fn list_entity_proposals(
        &self,
        ctx: Context<'_>,
        req: ListEntityProposalsRequest,
    ) -> Result<EntityProposalListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let proposals = SyncProposal::find_pending_for_entity(req.entity_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EntityProposalListResult {
            proposals: proposals
                .into_iter()
                .map(|p| EntityProposalResult {
                    id: p.id.into_uuid(),
                    batch_id: p.batch_id.into_uuid(),
                    operation: p.operation,
                    status: p.status,
                    entity_type: p.entity_type,
                    draft_entity_id: p.draft_entity_id,
                    target_entity_id: p.target_entity_id,
                    reason: p.reason,
                    created_at: p.created_at.to_rfc3339(),
                })
                .collect(),
        })
    }

    async fn approve_proposal(
        &self,
        ctx: Context<'_>,
        req: ApproveProposalRequest,
    ) -> Result<ProposalResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let handler = PostProposalHandler;
        let proposal = ctx
            .run(|| async {
                proposal_actions::approve_proposal(
                    SyncProposalId::from_uuid(req.proposal_id),
                    user.member_id,
                    &handler,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(ProposalResult {
            id: proposal.id.into_uuid(),
            batch_id: proposal.batch_id.into_uuid(),
            operation: proposal.operation,
            status: proposal.status,
        })
    }

    async fn reject_proposal(
        &self,
        ctx: Context<'_>,
        req: RejectProposalRequest,
    ) -> Result<ProposalResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let handler = PostProposalHandler;
        let proposal = ctx
            .run(|| async {
                proposal_actions::reject_proposal(
                    SyncProposalId::from_uuid(req.proposal_id),
                    user.member_id,
                    &handler,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(ProposalResult {
            id: proposal.id.into_uuid(),
            batch_id: proposal.batch_id.into_uuid(),
            operation: proposal.operation,
            status: proposal.status,
        })
    }

    async fn approve_batch(
        &self,
        ctx: Context<'_>,
        req: ApproveBatchRequest,
    ) -> Result<BatchResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let handler = PostProposalHandler;
        let batch = ctx
            .run(|| async {
                proposal_actions::approve_batch(
                    SyncBatchId::from_uuid(req.batch_id),
                    user.member_id,
                    &handler,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(BatchResult {
            id: batch.id.into_uuid(),
            resource_type: batch.resource_type,
            status: batch.status,
            proposal_count: batch.proposal_count,
        })
    }

    async fn reject_batch(
        &self,
        ctx: Context<'_>,
        req: RejectBatchRequest,
    ) -> Result<BatchResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let handler = PostProposalHandler;
        let batch = ctx
            .run(|| async {
                proposal_actions::reject_batch(
                    SyncBatchId::from_uuid(req.batch_id),
                    user.member_id,
                    &handler,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(BatchResult {
            id: batch.id.into_uuid(),
            resource_type: batch.resource_type,
            status: batch.status,
            proposal_count: batch.proposal_count,
        })
    }
}
