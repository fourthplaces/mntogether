//! Domain Approval effects
//!
//! Effects watch FACT events and call handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.
//!
//! Flow:
//!   GraphQL → assess_website action
//!     - Fresh research exists → generates assessment synchronously (no events)
//!     - Stale/missing research → WebsiteResearchCreated (triggers async cascade)
//!
//! Async cascade (only when research needs to be created):
//!   WebsiteResearchCreated → handle_conduct_searches → ResearchSearchesCompleted
//!   ResearchSearchesCompleted → handle_generate_assessment → WebsiteAssessmentCompleted

pub mod assessment;
pub mod search;

use seesaw_core::effect;
use std::sync::Arc;

use crate::common::AppState;
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::kernel::ServerDeps;

pub use assessment::handle_generate_assessment;
pub use search::handle_conduct_searches;

/// Build the domain approval effect handler.
///
/// This effect watches FACT events and calls handlers directly for cascading.
/// No *Requested events - the effect IS the cascade controller.
pub fn domain_approval_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<DomainApprovalEvent>().run(|event: Arc<DomainApprovalEvent>, ctx| async move {
        match event.as_ref() {
            // =================================================================
            // Cascade: WebsiteResearchCreated → conduct searches
            // =================================================================
            DomainApprovalEvent::WebsiteResearchCreated {
                research_id,
                website_id,
                job_id,
                requested_by,
                ..
            } => {
                handle_conduct_searches(*research_id, *website_id, *job_id, *requested_by, &ctx)
                    .await
            }

            // =================================================================
            // Cascade: ResearchSearchesCompleted → generate assessment
            // =================================================================
            DomainApprovalEvent::ResearchSearchesCompleted {
                research_id,
                website_id,
                job_id,
                requested_by,
                ..
            } => {
                handle_generate_assessment(*research_id, *website_id, *job_id, *requested_by, &ctx)
                    .await
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            DomainApprovalEvent::WebsiteResearchFailed { .. }
            | DomainApprovalEvent::ResearchSearchesFailed { .. }
            | DomainApprovalEvent::WebsiteAssessmentCompleted { .. }
            | DomainApprovalEvent::AssessmentGenerationFailed { .. } => Ok(()),
        }
    })
}
