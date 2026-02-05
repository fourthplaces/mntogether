//! Website Approval effects
//!
//! Event-driven pipeline:
//!   WebsiteResearchCreated → conduct_searches → generate_assessment (queued)
//!   ResearchSearchesCompleted → generate_assessment (retry path, queued)

use std::time::Duration;

use seesaw_core::effect;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, JobId, MemberId, WebsiteId};

use crate::domains::website_approval::actions;
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::ServerDeps;

/// Build the website approval research effect.
///
/// WebsiteResearchCreated → conduct searches + generate assessment (queued)
pub fn website_research_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteApprovalEvent>()
        .extract(|event| match event {
            WebsiteApprovalEvent::WebsiteResearchCreated {
                research_id,
                website_id,
                job_id,
                requested_by,
                ..
            } => Some((*research_id, *website_id, *job_id, *requested_by)),
            _ => None,
        })
        .id("website_research_conduct")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(120))
        .then(
            |(research_id, website_id, job_id, requested_by): (Uuid, WebsiteId, JobId, MemberId),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
                info!(
                    research_id = %research_id,
                    website_id = %website_id,
                    "Conducting searches for research (queued)"
                );

                let result =
                    actions::conduct_searches(research_id, website_id, ctx.deps()).await?;

                info!(
                    research_id = %research_id,
                    total_queries = result.total_queries,
                    total_results = result.total_results,
                    "Searches completed"
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
            },
        )
}

/// Retry path: ResearchSearchesCompleted → generate assessment (queued)
pub fn website_research_retry_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteApprovalEvent>()
        .extract(|event| match event {
            WebsiteApprovalEvent::ResearchSearchesCompleted {
                research_id,
                website_id,
                job_id,
                requested_by,
                ..
            } => Some((*research_id, *website_id, *job_id, *requested_by)),
            _ => None,
        })
        .id("website_research_assessment_retry")
        .queued()
        .retry(2)
        .timeout(Duration::from_secs(60))
        .then(
            |(research_id, website_id, job_id, requested_by): (Uuid, WebsiteId, JobId, MemberId),
             ctx: seesaw_core::EffectContext<AppState, ServerDeps>| async move {
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
            },
        )
}

/// Composite approval effect combining both research paths.
pub fn website_approval_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    seesaw_core::effect::group([
        website_research_effect(),
        website_research_retry_effect(),
    ])
}
