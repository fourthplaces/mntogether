//! Search effect handler - thin wrapper around search action
//!
//! Effect handlers watch FACT events and route to actions.
//! This handler is called when WebsiteResearchCreated is emitted.

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::website_approval::actions::{self, SearchResult};
use crate::kernel::ServerDeps;
use anyhow::Result;
use seesaw_core::EffectContext;
use uuid::Uuid;

/// Effect handler for WebsiteResearchCreated cascade.
///
/// Calls the conduct_searches action and returns the result.
pub async fn handle_conduct_searches(
    research_id: Uuid,
    website_id: WebsiteId,
    _job_id: JobId,
    _requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<SearchResult> {
    // Call action and return result
    actions::conduct_searches(research_id, website_id, ctx.deps()).await
}
