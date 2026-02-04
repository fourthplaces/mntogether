//! Assessment effect handler - thin wrapper around assessment action
//!
//! Effect handlers watch FACT events and route to actions.
//! This handler is called when ResearchSearchesCompleted is emitted.

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::website_approval::actions;
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::ServerDeps;
use anyhow::Result;
use seesaw_core::EffectContext;
use uuid::Uuid;

/// Effect handler for ResearchSearchesCompleted cascade.
///
/// Thin wrapper that calls the generate_assessment action and emits completion event.
pub async fn handle_generate_assessment(
    research_id: Uuid,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    // Call action
    let assessment =
        actions::generate_assessment(research_id, website_id, job_id, requested_by, ctx).await?;

    // Emit event
    ctx.emit(WebsiteApprovalEvent::WebsiteAssessmentCompleted {
        website_id,
        job_id,
        assessment_id: assessment.id,
        recommendation: assessment.recommendation.clone(),
        confidence_score: assessment.confidence_score,
        organization_name: assessment.organization_name.clone(),
    });

    Ok(())
}
