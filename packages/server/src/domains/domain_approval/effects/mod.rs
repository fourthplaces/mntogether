pub mod assessment;
pub mod research;
pub mod search;

use crate::domains::domain_approval::commands::DomainApprovalCommand;
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::domains::listings::effects::deps::ServerDeps;
use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

pub use assessment::AssessmentEffect;
pub use research::ResearchEffect;
pub use search::SearchEffect;

/// Composite Effect - Routes commands to specialized effects
///
/// This effect is a thin orchestration layer that dispatches commands to specialized effects.
pub struct DomainApprovalCompositeEffect {
    research: ResearchEffect,
    search: SearchEffect,
    assessment: AssessmentEffect,
}

impl DomainApprovalCompositeEffect {
    pub fn new() -> Self {
        Self {
            research: ResearchEffect,
            search: SearchEffect,
            assessment: AssessmentEffect,
        }
    }
}

#[async_trait]
impl Effect<DomainApprovalCommand, ServerDeps> for DomainApprovalCompositeEffect {
    type Event = DomainApprovalEvent;

    async fn execute(
        &self,
        cmd: DomainApprovalCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<DomainApprovalEvent> {
        // Route commands to specialized effects
        match &cmd {
            DomainApprovalCommand::FetchOrCreateResearch { .. } => {
                self.research.execute(cmd, ctx).await
            }
            DomainApprovalCommand::ConductResearchSearches { .. } => {
                self.search.execute(cmd, ctx).await
            }
            DomainApprovalCommand::GenerateAssessmentFromResearch { .. } => {
                self.assessment.execute(cmd, ctx).await
            }
        }
    }
}
