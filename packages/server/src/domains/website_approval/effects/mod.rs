//! Website Approval effects
//!
//! Event-driven pipeline - each step does ONE thing:
//!   WebsiteResearchCreated → conduct_searches → generate_assessment
//!   ResearchSearchesCompleted → generate_assessment (for retries)

use seesaw_core::{effect, EffectContext};
use tracing::info;

use crate::common::AppState;
use crate::domains::website_approval::actions;
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::ServerDeps;

/// Build the website approval effect handler.
///
/// Each match arm calls an action directly - no handler indirection.
/// Errors propagate to global on_error() handler.
pub fn website_approval_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteApprovalEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Step 1: WebsiteResearchCreated → conduct searches → assessment
                // =================================================================
                WebsiteApprovalEvent::WebsiteResearchCreated {
                    research_id,
                    website_id,
                    job_id,
                    requested_by,
                    ..
                } => {
                    info!(
                        research_id = %research_id,
                        website_id = %website_id,
                        "Conducting searches for research"
                    );

                    let result =
                        actions::conduct_searches(*research_id, *website_id, ctx.deps()).await?;

                    info!(
                        research_id = %research_id,
                        total_queries = result.total_queries,
                        total_results = result.total_results,
                        "Searches completed"
                    );

                    let assessment = actions::generate_assessment(
                        *research_id,
                        *website_id,
                        *job_id,
                        *requested_by,
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

                // =================================================================
                // Step 2: ResearchSearchesCompleted → generate assessment (retry path)
                // =================================================================
                WebsiteApprovalEvent::ResearchSearchesCompleted {
                    research_id,
                    website_id,
                    job_id,
                    requested_by,
                    ..
                } => {
                    info!(
                        research_id = %research_id,
                        website_id = %website_id,
                        "Generating assessment"
                    );

                    let assessment = actions::generate_assessment(
                        *research_id,
                        *website_id,
                        *job_id,
                        *requested_by,
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

                // =================================================================
                // Terminal events - no cascade needed
                // =================================================================
                WebsiteApprovalEvent::WebsiteResearchFailed { .. }
                | WebsiteApprovalEvent::ResearchSearchesFailed { .. }
                | WebsiteApprovalEvent::WebsiteAssessmentCompleted { .. }
                | WebsiteApprovalEvent::AssessmentGenerationFailed { .. } => Ok(()),
            }
        },
    )
}
