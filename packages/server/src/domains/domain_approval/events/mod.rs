//! Domain Approval events - FACT EVENTS ONLY
//!
//! Events are immutable facts about what happened. Effects watch these
//! and call handlers directly for cascade workflows (no *Requested events).
//!
//! Flow:
//!   GraphQL → assess_website action
//!     - Fresh research exists → generates assessment synchronously → returns immediately
//!     - Stale/missing research → WebsiteResearchCreated (triggers async cascade)
//!
//! Async cascade (stale/missing research):
//!   WebsiteResearchCreated → handle_conduct_searches → ResearchSearchesCompleted
//!   ResearchSearchesCompleted → handle_generate_assessment → WebsiteAssessmentCompleted

use crate::common::{JobId, MemberId, WebsiteId};
use uuid::Uuid;

/// Domain approval events - FACT EVENTS ONLY
#[derive(Debug, Clone)]
pub enum DomainApprovalEvent {
    // ========================================================================
    // Research Phase Events
    // ========================================================================
    /// New research record created (homepage scraped and stored)
    /// Triggers async search cascade when fresh research doesn't exist.
    WebsiteResearchCreated {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        homepage_url: String,
        requested_by: MemberId,
    },

    /// Failed to fetch or create research
    WebsiteResearchFailed {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    // ========================================================================
    // Search Phase Events
    // ========================================================================
    /// All Tavily searches completed and results stored
    ResearchSearchesCompleted {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        total_queries: usize,
        total_results: usize,
        requested_by: MemberId,
    },

    /// Failed to conduct searches
    ResearchSearchesFailed {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    // ========================================================================
    // Assessment Phase Events
    // ========================================================================
    /// AI assessment generated and stored
    WebsiteAssessmentCompleted {
        website_id: WebsiteId,
        job_id: JobId,
        assessment_id: Uuid,
        recommendation: String,
        confidence_score: Option<f64>,
        organization_name: Option<String>,
    },

    /// Failed to generate assessment
    AssessmentGenerationFailed {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },
}
