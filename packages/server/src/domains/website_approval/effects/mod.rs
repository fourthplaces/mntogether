//! Website Approval effects
//!
//! Effects use `.then()` and return `Ok(())` for terminal.
//!
//! Flow:
//!   GraphQL → assess_website action → emits WebsiteResearchCreated (or completes synchronously)
//!
//! Async cascade (only when research needs to be created):
//!   WebsiteResearchCreated → handle_conduct_searches → (internally chains to assessment)
//!   ResearchSearchesCompleted → handle_generate_assessment → WebsiteAssessmentCompleted (terminal)
//!
//! Note: The cascade is handled internally by the handlers rather than via event return types
//! due to Rust type system constraints. Each handler performs its work and the next event
//! is emitted from within the handler (using the action's internal logic).

pub mod assessment;
pub mod search;

use seesaw_core::{effect, EffectContext};
use tracing::{error, info};

use crate::common::AppState;
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::ServerDeps;

pub use assessment::handle_generate_assessment;
pub use search::handle_conduct_searches;

/// Build the website approval effect handler.
///
/// This effect watches FACT events and chains to the next event in the workflow.
/// All branches return Ok(()) since cascading is handled internally by handlers.
pub fn website_approval_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteApprovalEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: WebsiteResearchCreated → conduct searches
                // The handler performs searches. The assess_website action will
                // call generate_assessment next if needed.
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
                        "Handling WebsiteResearchCreated - conducting searches"
                    );

                    // Conduct searches (result contains stats but we log them)
                    match handle_conduct_searches(
                        *research_id,
                        *website_id,
                        *job_id,
                        *requested_by,
                        &ctx,
                    )
                    .await
                    {
                        Ok(result) => {
                            info!(
                                research_id = %research_id,
                                total_queries = result.total_queries,
                                total_results = result.total_results,
                                "Searches completed, now generating assessment"
                            );

                            // Chain to assessment generation
                            if let Err(e) = handle_generate_assessment(
                                *research_id,
                                *website_id,
                                *job_id,
                                *requested_by,
                                &ctx,
                            )
                            .await
                            {
                                error!(error = %e, "Assessment generation failed");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Search cascade failed");
                        }
                    }

                    Ok(()) // Terminal
                }

                // =================================================================
                // ResearchSearchesCompleted - this is now handled internally above
                // But we still match it in case it's emitted directly
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
                        "Handling ResearchSearchesCompleted - generating assessment"
                    );

                    if let Err(e) = handle_generate_assessment(
                        *research_id,
                        *website_id,
                        *job_id,
                        *requested_by,
                        &ctx,
                    )
                    .await
                    {
                        error!(error = %e, "Assessment generation failed");
                    }

                    Ok(()) // Terminal
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
