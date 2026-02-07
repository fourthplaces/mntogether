//! Website research workflow
//!
//! Durable workflow that orchestrates website research and assessment:
//! 1. Conduct searches (build queries + execute + store)
//! 2. Generate AI assessment

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{JobId, MemberId, WebsiteId};
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

        // Single durable block: conduct searches → generate assessment
        let result = ctx
            .run(|| async {
                // Step 1: Conduct all searches
                let search_result =
                    approval::conduct_searches(request.research_id, website_id, &self.deps)
                        .await?;

                tracing::info!(
                    research_id = %request.research_id,
                    total_queries = search_result.total_queries,
                    total_results = search_result.total_results,
                    "Searches completed, generating assessment"
                );

                // Step 2: Generate AI assessment
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

        Ok(result)
    }
}
