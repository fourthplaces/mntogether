use std::sync::Arc;

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::common::MemberId;
use crate::domains::curator::activities::refine_proposal::{
    refine_proposal_from_comment, RefineResult,
};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineProposalRequest {
    pub proposal_id: Uuid,
    pub comment: String,
    pub author_id: Uuid,
}
impl_restate_serde!(RefineProposalRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineProposalResult {
    pub status: String,
}
impl_restate_serde!(RefineProposalResult);

#[restate_sdk::workflow]
#[name = "RefineProposalWorkflow"]
pub trait RefineProposalWorkflow {
    async fn run(req: RefineProposalRequest) -> Result<RefineProposalResult, HandlerError>;
}

pub struct RefineProposalWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl RefineProposalWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl RefineProposalWorkflow for RefineProposalWorkflowImpl {
    async fn run(
        &self,
        _ctx: WorkflowContext<'_>,
        req: RefineProposalRequest,
    ) -> Result<RefineProposalResult, HandlerError> {
        info!(
            proposal_id = %req.proposal_id,
            "Starting proposal refinement"
        );

        let result = refine_proposal_from_comment(
            req.proposal_id,
            &req.comment,
            MemberId::from(req.author_id),
            &self.deps,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let status = match result {
            RefineResult::Revised => "revised",
            RefineResult::MaxRevisionsReached => "max_revisions_reached",
        };

        info!(
            proposal_id = %req.proposal_id,
            status = status,
            "Proposal refinement complete"
        );

        Ok(RefineProposalResult {
            status: status.to_string(),
        })
    }
}
