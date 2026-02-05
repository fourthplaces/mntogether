//! Assessment effect handler - thin wrapper around assessment action
//!
//! Effect handlers watch FACT events and route to actions.
//! This handler is called when ResearchSearchesCompleted is emitted.

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::website::models::WebsiteAssessment;
use crate::domains::website_approval::actions;
use crate::kernel::ServerDeps;
use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

/// Effect handler for ResearchSearchesCompleted cascade.
///
/// Calls the generate_assessment action and returns the assessment.
pub async fn handle_generate_assessment(
    research_id: Uuid,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<WebsiteAssessment> {
    // Call action
    let assessment =
        actions::generate_assessment(research_id, website_id, job_id, requested_by, ctx.deps()).await?;

    info!(
        assessment_id = %assessment.id,
        website_id = %website_id,
        recommendation = %assessment.recommendation,
        "Assessment completed"
    );

    Ok(assessment)
}
