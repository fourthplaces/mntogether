//! Sync service (stateless)
//!
//! Batch/proposal management for data synchronization.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{OrganizationId, SyncBatchId, SyncProposalId, WebsiteId};
use crate::domains::agents::models::Agent;
use crate::domains::organization::models::Organization;
use crate::domains::posts::models::Post;
use crate::domains::sync::activities::proposal_actions;
use crate::domains::sync::{SyncBatch, SyncProposal, SyncProposalMergeSource};
use crate::domains::website::models::Website;
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
    pub source_id: Option<Uuid>,
    pub source_name: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub proposal_count: i32,
    pub approved_count: i32,
    pub rejected_count: i32,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

impl_restate_serde!(BatchResult);

impl BatchResult {
    fn from_model(b: SyncBatch, source_name: Option<String>) -> Self {
        Self {
            id: b.id.into_uuid(),
            resource_type: b.resource_type,
            source_id: b.source_id,
            source_name,
            status: b.status,
            summary: b.summary,
            proposal_count: b.proposal_count,
            approved_count: b.approved_count,
            rejected_count: b.rejected_count,
            created_at: b.created_at.to_rfc3339(),
            reviewed_at: b.reviewed_at.map(|t| t.to_rfc3339()),
        }
    }
}

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
    pub entity_type: String,
    pub draft_entity_id: Option<Uuid>,
    pub target_entity_id: Option<Uuid>,
    pub reason: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub draft_title: Option<String>,
    pub target_title: Option<String>,
    pub merge_source_ids: Vec<Uuid>,
    pub merge_source_titles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "curator_reasoning")]
    pub consultant_reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_urls: Option<Vec<String>>,
    pub revision_count: i32,
}

impl_restate_serde!(ProposalResult);

impl ProposalResult {
    fn from_model(p: SyncProposal) -> Self {
        Self {
            id: p.id.into_uuid(),
            batch_id: p.batch_id.into_uuid(),
            operation: p.operation,
            status: p.status,
            entity_type: p.entity_type,
            draft_entity_id: p.draft_entity_id,
            target_entity_id: p.target_entity_id,
            reason: p.reason,
            reviewed_by: p.reviewed_by,
            reviewed_at: p.reviewed_at.map(|t| t.to_rfc3339()),
            created_at: p.created_at.to_rfc3339(),
            draft_title: None,
            target_title: None,
            merge_source_ids: vec![],
            merge_source_titles: vec![],
            relevance_score: None,
            consultant_reasoning: p.consultant_reasoning,
            confidence: p.confidence,
            source_urls: p.source_urls,
            revision_count: p.revision_count,
        }
    }
}

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
    async fn list_proposals(req: ListProposalsRequest) -> Result<ProposalListResult, HandlerError>;
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

    async fn lookup_source_name(batch: &SyncBatch, pool: &sqlx::PgPool) -> Option<String> {
        let source_id = batch.source_id?;
        if batch.resource_type == "curator" || batch.resource_type == "organization" {
            if let Ok(org) =
                Organization::find_by_id(OrganizationId::from(source_id), pool).await
            {
                return Some(org.name);
            }
        }
        if let Ok(website) = Website::find_by_id(WebsiteId::from_uuid(source_id), pool).await {
            return Some(website.domain);
        }
        if let Ok(agent) = Agent::find_by_id(source_id, pool).await {
            return Some(agent.display_name);
        }
        None
    }
}

impl SyncService for SyncServiceImpl {
    async fn list_batches(
        &self,
        ctx: Context<'_>,
        req: ListBatchesRequest,
    ) -> Result<BatchListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        let limit = req.limit.unwrap_or(20);
        let batches = match req.status.as_deref() {
            Some("pending") => SyncBatch::find_pending(pool).await,
            _ => SyncBatch::find_recent(limit, pool).await,
        }
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Look up source names: try org (curator), website, then agent
        let mut source_name_map: std::collections::HashMap<Uuid, String> =
            std::collections::HashMap::new();
        for b in &batches {
            if let Some(source_id) = b.source_id {
                if !source_name_map.contains_key(&source_id) {
                    if b.resource_type == "curator" || b.resource_type == "organization" {
                        if let Ok(org) =
                            Organization::find_by_id(OrganizationId::from(source_id), pool).await
                        {
                            source_name_map.insert(source_id, org.name);
                        }
                    } else if let Ok(website) =
                        Website::find_by_id(WebsiteId::from_uuid(source_id), pool).await
                    {
                        source_name_map.insert(source_id, website.domain);
                    } else if let Ok(agent) = Agent::find_by_id(source_id, pool).await {
                        source_name_map.insert(source_id, agent.display_name);
                    }
                }
            }
        }

        Ok(BatchListResult {
            batches: batches
                .into_iter()
                .map(|b| {
                    let source_name = b.source_id.and_then(|id| source_name_map.get(&id).cloned());
                    BatchResult::from_model(b, source_name)
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
        let pool = &self.deps.db_pool;

        let batch = SyncBatch::find_by_id(SyncBatchId::from_uuid(req.id), pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Batch not found"))?;

        let source_name = Self::lookup_source_name(&batch, pool).await;

        Ok(BatchResult::from_model(batch, source_name))
    }

    async fn list_proposals(
        &self,
        ctx: Context<'_>,
        req: ListProposalsRequest,
    ) -> Result<ProposalListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        let proposals = SyncProposal::find_by_batch(SyncBatchId::from_uuid(req.batch_id), pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Collect all entity IDs we need titles for
        let mut entity_ids: Vec<Uuid> = Vec::new();
        for p in &proposals {
            if let Some(id) = p.draft_entity_id {
                entity_ids.push(id);
            }
            if let Some(id) = p.target_entity_id {
                entity_ids.push(id);
            }
        }

        // Batch-load merge sources for all proposals
        let mut merge_sources_by_proposal: std::collections::HashMap<Uuid, Vec<Uuid>> =
            std::collections::HashMap::new();
        for p in &proposals {
            let sources = SyncProposalMergeSource::find_by_proposal(p.id, pool)
                .await
                .unwrap_or_default();
            let source_ids: Vec<Uuid> = sources.iter().map(|s| s.source_entity_id).collect();
            entity_ids.extend(&source_ids);
            merge_sources_by_proposal.insert(p.id.into_uuid(), source_ids);
        }

        // Batch-load all post titles + scores (includes soft-deleted posts for display)
        entity_ids.sort();
        entity_ids.dedup();
        let title_score_rows = Post::find_titles_and_scores_by_ids(&entity_ids, pool)
            .await
            .unwrap_or_default();
        let title_map: std::collections::HashMap<Uuid, String> = title_score_rows
            .iter()
            .map(|(id, title, _)| (*id, title.clone()))
            .collect();
        let score_map: std::collections::HashMap<Uuid, Option<i32>> = title_score_rows
            .into_iter()
            .map(|(id, _, score)| (id, score))
            .collect();

        Ok(ProposalListResult {
            proposals: proposals
                .into_iter()
                .map(|p| {
                    let pid = p.id.into_uuid();
                    let merge_ids = merge_sources_by_proposal
                        .get(&pid)
                        .cloned()
                        .unwrap_or_default();
                    let merge_titles: Vec<String> = merge_ids
                        .iter()
                        .filter_map(|id| title_map.get(id).cloned())
                        .collect();

                    // Use draft entity score if available, otherwise target entity score
                    let relevance_score = p
                        .draft_entity_id
                        .and_then(|id| score_map.get(&id).copied().flatten())
                        .or_else(|| {
                            p.target_entity_id
                                .and_then(|id| score_map.get(&id).copied().flatten())
                        });

                    ProposalResult {
                        id: pid,
                        batch_id: p.batch_id.into_uuid(),
                        operation: p.operation,
                        status: p.status,
                        entity_type: p.entity_type,
                        draft_entity_id: p.draft_entity_id,
                        target_entity_id: p.target_entity_id,
                        reason: p.reason,
                        reviewed_by: p.reviewed_by,
                        reviewed_at: p.reviewed_at.map(|t| t.to_rfc3339()),
                        created_at: p.created_at.to_rfc3339(),
                        draft_title: p.draft_entity_id.and_then(|id| title_map.get(&id).cloned()),
                        target_title: p
                            .target_entity_id
                            .and_then(|id| title_map.get(&id).cloned()),
                        merge_source_ids: merge_ids,
                        merge_source_titles: merge_titles,
                        relevance_score,
                        consultant_reasoning: p.consultant_reasoning,
                        confidence: p.confidence,
                        source_urls: p.source_urls,
                        revision_count: p.revision_count,
                    }
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

        let proposal = ctx
            .run(|| async {
                proposal_actions::approve_proposal_auto(
                    SyncProposalId::from_uuid(req.proposal_id),
                    user.member_id,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(ProposalResult::from_model(proposal))
    }

    async fn reject_proposal(
        &self,
        ctx: Context<'_>,
        req: RejectProposalRequest,
    ) -> Result<ProposalResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let proposal = ctx
            .run(|| async {
                proposal_actions::reject_proposal_auto(
                    SyncProposalId::from_uuid(req.proposal_id),
                    user.member_id,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(ProposalResult::from_model(proposal))
    }

    async fn approve_batch(
        &self,
        ctx: Context<'_>,
        req: ApproveBatchRequest,
    ) -> Result<BatchResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let batch = ctx
            .run(|| async {
                proposal_actions::approve_batch_auto(
                    SyncBatchId::from_uuid(req.batch_id),
                    user.member_id,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        let source_name = Self::lookup_source_name(&batch, &self.deps.db_pool).await;
        Ok(BatchResult::from_model(batch, source_name))
    }

    async fn reject_batch(
        &self,
        ctx: Context<'_>,
        req: RejectBatchRequest,
    ) -> Result<BatchResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let batch = ctx
            .run(|| async {
                proposal_actions::reject_batch_auto(
                    SyncBatchId::from_uuid(req.batch_id),
                    user.member_id,
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        let source_name = Self::lookup_source_name(&batch, &self.deps.db_pool).await;
        Ok(BatchResult::from_model(batch, source_name))
    }
}
