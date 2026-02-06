//! Website Approval events - FACT EVENTS ONLY
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

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{JobId, MemberId, WebsiteId};

/// Website approval events - FACT EVENTS ONLY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebsiteApprovalEvent {
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

    // ========================================================================
    // Fan-out Search Events (batch/join pipeline)
    // ========================================================================
    /// Single search query enqueued
    ///
    /// Emitted as a batch by `prepare_searches` effect.
    /// Picked up by `execute_search` effect (parallel per query).
    ResearchSearchEnqueued {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        query: String,
    },

    /// Single search query completed
    ///
    /// Emitted by `execute_search` effect after Tavily search.
    /// Joined by `join_searches` effect.
    ResearchSearchCompleted {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        query: String,
        result_count: usize,
    },

    /// Assessment generation enqueued (after all searches joined)
    ///
    /// Emitted by `join_searches` effect. Picked up by `generate_assessment` effect.
    AssessmentGenerationEnqueued {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },
}
