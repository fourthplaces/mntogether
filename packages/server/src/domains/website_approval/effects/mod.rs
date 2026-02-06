//! Website Approval effects with fan-out/join search pipeline
//!
//! Pipeline:
//! ```text
//! WebsiteResearchCreated       → prepare_searches      → Batch([ResearchSearchEnqueued x3])
//! ResearchSearchEnqueued       → execute_search         → ResearchSearchCompleted [parallel]
//! ResearchSearchCompleted      → join_searches (join)   → AssessmentGenerationEnqueued
//! AssessmentGenerationEnqueued → generate_assessment     → terminal
//! ResearchSearchesCompleted    → assessment_retry        → terminal (retry path)
//! ```

use anyhow::Result;
use seesaw_core::{effect, effects, EffectContext, Emit};
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::website::models::WebsiteResearch;
use crate::domains::website_approval::actions;
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::ServerDeps;

#[effects]
pub mod handlers {
    use super::*;

    // =========================================================================
    // Step 1: Build search queries → fan-out
    // =========================================================================

    #[effect(
        on = [WebsiteApprovalEvent::WebsiteResearchCreated],
        extract(research_id, website_id, job_id, requested_by),
        id = "prepare_searches",
        retry = 2,
        timeout_secs = 30
    )]
    async fn prepare_searches(
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<Emit<WebsiteApprovalEvent>> {
        info!(
            research_id = %research_id,
            website_id = %website_id,
            "Preparing search queries for research"
        );

        let queries =
            actions::build_search_queries(research_id, website_id, ctx.deps()).await?;

        info!(
            research_id = %research_id,
            query_count = queries.len(),
            "Fanning out search queries"
        );

        Ok(Emit::Batch(
            queries
                .into_iter()
                .map(|query| WebsiteApprovalEvent::ResearchSearchEnqueued {
                    research_id,
                    website_id,
                    job_id,
                    requested_by,
                    query,
                })
                .collect(),
        ))
    }

    // =========================================================================
    // Step 2: Execute individual search (parallel per query)
    // =========================================================================

    #[effect(
        on = [WebsiteApprovalEvent::ResearchSearchEnqueued],
        extract(research_id, website_id, job_id, requested_by, query),
        id = "execute_search",
        retry = 2,
        timeout_secs = 30
    )]
    async fn execute_search(
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        query: String,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<WebsiteApprovalEvent> {
        info!(research_id = %research_id, query = %query, "Executing search");

        let result_count =
            actions::execute_and_store_search(research_id, &query, ctx.deps()).await?;

        info!(
            research_id = %research_id,
            query = %query,
            result_count = result_count,
            "Search completed"
        );

        Ok(WebsiteApprovalEvent::ResearchSearchCompleted {
            research_id,
            website_id,
            job_id,
            requested_by,
            query,
            result_count,
        })
    }

    // =========================================================================
    // Step 3: Join all searches → trigger assessment
    // =========================================================================

    #[effect(on = WebsiteApprovalEvent, join, id = "join_searches")]
    async fn join_searches(
        events: Vec<WebsiteApprovalEvent>,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<WebsiteApprovalEvent> {
        // Extract common fields from first event
        let first = events.first().expect("batch must have items");
        let (research_id, website_id, job_id, requested_by) = match first {
            WebsiteApprovalEvent::ResearchSearchCompleted {
                research_id,
                website_id,
                job_id,
                requested_by,
                ..
            } => (*research_id, *website_id, *job_id, *requested_by),
            _ => unreachable!("join should only receive ResearchSearchCompleted events"),
        };

        let total_results: usize = events
            .iter()
            .filter_map(|e| match e {
                WebsiteApprovalEvent::ResearchSearchCompleted {
                    result_count, ..
                } => Some(*result_count),
                _ => None,
            })
            .sum();

        info!(
            research_id = %research_id,
            total_results = total_results,
            search_count = events.len(),
            "All searches joined, marking research complete"
        );

        // Mark research as tavily-complete
        let research =
            WebsiteResearch::find_latest_by_website_id(website_id.into(), &ctx.deps().db_pool)
                .await?
                .expect("research exists");
        research.mark_tavily_complete(&ctx.deps().db_pool).await?;

        Ok(WebsiteApprovalEvent::AssessmentGenerationEnqueued {
            research_id,
            website_id,
            job_id,
            requested_by,
        })
    }

    // =========================================================================
    // Step 4: Generate assessment
    // =========================================================================

    #[effect(
        on = [WebsiteApprovalEvent::AssessmentGenerationEnqueued],
        extract(research_id, website_id, job_id, requested_by),
        id = "generate_assessment",
        retry = 2,
        timeout_secs = 120
    )]
    async fn generate_assessment(
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        info!(
            research_id = %research_id,
            website_id = %website_id,
            "Generating assessment"
        );

        let assessment = actions::generate_assessment(
            research_id,
            website_id,
            job_id,
            requested_by,
            ctx.deps(),
        )
        .await?;

        info!(
            assessment_id = %assessment.id,
            recommendation = %assessment.recommendation,
            "Assessment completed"
        );
        Ok(())
    }

    // =========================================================================
    // Retry path: ResearchSearchesCompleted → assessment (legacy path)
    // =========================================================================

    #[effect(
        on = [WebsiteApprovalEvent::ResearchSearchesCompleted],
        extract(research_id, website_id, job_id, requested_by),
        id = "website_research_assessment_retry",
        retry = 2,
        timeout_secs = 60
    )]
    async fn assessment_retry(
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        info!(
            research_id = %research_id,
            website_id = %website_id,
            "Generating assessment (retry path, queued)"
        );

        let assessment = actions::generate_assessment(
            research_id,
            website_id,
            job_id,
            requested_by,
            ctx.deps(),
        )
        .await?;

        info!(
            assessment_id = %assessment.id,
            recommendation = %assessment.recommendation,
            "Assessment completed"
        );
        Ok(())
    }
}
