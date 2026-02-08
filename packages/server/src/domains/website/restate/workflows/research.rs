//! Website research workflow
//!
//! Durable workflow that orchestrates website research and assessment:
//! 1. Conduct searches (build queries + execute + store)
//! 2. Generate AI assessment
//!
//! Each step is a separate ctx.run() block so Restate journals intermediate
//! results and won't re-execute completed steps on retry.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{EmptyRequest, JobId, MemberId, WebsiteId};
use crate::domains::website::activities::approval;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteResearchRequest {
    pub research_id: Uuid,
    pub website_id: Uuid,
    pub requested_by: Uuid,
}

impl_restate_serde!(WebsiteResearchRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteResearchResult {
    pub assessment_id: Option<Uuid>,
    pub recommendation: Option<String>,
    pub status: String,
}

impl_restate_serde!(WebsiteResearchResult);

#[restate_sdk::workflow]
pub trait WebsiteResearchWorkflow {
    async fn run(request: WebsiteResearchRequest) -> Result<WebsiteResearchResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct WebsiteResearchWorkflowImpl {
    deps: std::sync::Arc<ServerDeps>,
}

impl WebsiteResearchWorkflowImpl {
    pub fn with_deps(deps: std::sync::Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl WebsiteResearchWorkflow for WebsiteResearchWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: WebsiteResearchRequest,
    ) -> Result<WebsiteResearchResult, HandlerError> {
        let website_id = WebsiteId::from_uuid(request.website_id);
        let requested_by = MemberId::from_uuid(request.requested_by);
        let job_id = JobId::new();

        tracing::info!(
            research_id = %request.research_id,
            website_id = %request.website_id,
            "Starting website research workflow"
        );

        // Step 1: Conduct all searches — journaled, won't re-run on replay
        ctx.set("status", "Researching website...".to_string());

        let search_result = ctx
            .run(|| async {
                approval::conduct_searches(request.research_id, website_id, &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        tracing::info!(
            research_id = %request.research_id,
            total_queries = search_result.total_queries,
            total_results = search_result.total_results,
            "Searches completed, generating assessment"
        );

        // Step 2: Generate AI assessment — if this fails, step 1 is replayed from journal
        ctx.set("status", "Generating assessment...".to_string());

        let result = ctx
            .run(|| async {
                let assessment = approval::generate_assessment(
                    request.research_id,
                    website_id,
                    job_id,
                    requested_by,
                    &self.deps,
                )
                .await?;

                tracing::info!(
                    assessment_id = %assessment.id,
                    recommendation = %assessment.recommendation,
                    "Assessment generated"
                );

                Ok(WebsiteResearchResult {
                    assessment_id: Some(assessment.id),
                    recommendation: Some(assessment.recommendation),
                    status: "completed".to_string(),
                })
            })
            .await?;

        ctx.set("status", "Completed".to_string());

        Ok(result)
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
