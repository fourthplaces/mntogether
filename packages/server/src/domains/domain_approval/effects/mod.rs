//! Domain Approval effects
//!
//! Effects are thin orchestration layers that dispatch events to handler functions.
//! All business logic lives in handler functions.

pub mod assessment;
pub mod research;
pub mod search;

use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::kernel::ServerDeps;
use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

pub use assessment::handle_generate_assessment;
pub use research::handle_assess_website;
pub use search::handle_conduct_searches;

/// Domain Approval Composite Effect - Handles DomainApprovalEvent request events
///
/// This effect is a thin orchestration layer that dispatches request events to handler functions.
/// Fact events should never reach this effect (they're outputs, not inputs).
pub struct DomainApprovalEffect;

impl DomainApprovalEffect {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DomainApprovalEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Effect<DomainApprovalEvent, ServerDeps> for DomainApprovalEffect {
    type Event = DomainApprovalEvent;

    async fn handle(
        &mut self,
        event: DomainApprovalEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<Option<DomainApprovalEvent>> {
        match event {
            // =================================================================
            // Request Events → Dispatch to Handlers
            // =================================================================
            DomainApprovalEvent::AssessWebsiteRequested {
                website_id,
                job_id,
                requested_by,
            } => handle_assess_website(website_id, job_id, requested_by, &ctx).await.map(Some),

            DomainApprovalEvent::ConductResearchSearchesRequested {
                research_id,
                website_id,
                job_id,
                requested_by,
            } => handle_conduct_searches(research_id, website_id, job_id, requested_by, &ctx).await.map(Some),

            DomainApprovalEvent::GenerateAssessmentFromResearchRequested {
                research_id,
                website_id,
                job_id,
                requested_by,
            } => handle_generate_assessment(research_id, website_id, job_id, requested_by, &ctx).await.map(Some),

            // =================================================================
            // Fact Events → Terminal, no follow-up needed
            // =================================================================
            DomainApprovalEvent::WebsiteResearchFound { .. }
            | DomainApprovalEvent::WebsiteResearchCreated { .. }
            | DomainApprovalEvent::WebsiteResearchFailed { .. }
            | DomainApprovalEvent::ResearchSearchesCompleted { .. }
            | DomainApprovalEvent::ResearchSearchesFailed { .. }
            | DomainApprovalEvent::WebsiteAssessmentCompleted { .. }
            | DomainApprovalEvent::AssessmentGenerationFailed { .. } => Ok(None),
        }
    }
}
