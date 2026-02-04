//! Search effect handler - thin wrapper around search action
//!
//! Effect handlers watch FACT events and route to actions.
//! This handler is called when WebsiteResearchCreated is emitted.

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::website_approval::actions;
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::ServerDeps;
use anyhow::Result;
use seesaw_core::EffectContext;
use uuid::Uuid;

/// Effect handler for WebsiteResearchCreated cascade.
///
/// Thin wrapper that calls the conduct_searches action and emits completion event.
pub async fn handle_conduct_searches(
    research_id: Uuid,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    // Call action
    let result = actions::conduct_searches(research_id, website_id, ctx).await?;

    // Emit event
    ctx.emit(WebsiteApprovalEvent::ResearchSearchesCompleted {
        research_id,
        website_id,
        job_id,
        total_queries: result.total_queries,
        total_results: result.total_results,
        requested_by,
    });

    Ok(())
}
