//! Domain Approval events
//!
//! Architecture (seesaw 0.3.0):
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...
//!
//! Event Chain:
//! 1. AssessWebsiteRequested → Effect → WebsiteResearchFound/WebsiteResearchCreated
//! 2. WebsiteResearchFound → InternalEdge → GenerateAssessmentFromResearchRequested
//! 3. WebsiteResearchCreated → InternalEdge → ConductResearchSearchesRequested
//! 4. ResearchSearchesCompleted → InternalEdge → GenerateAssessmentFromResearchRequested
//! 5. WebsiteAssessmentCompleted → [terminal]

use crate::common::{JobId, MemberId, WebsiteId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DomainApprovalEvent {
    // ========================================================================
    // Request Events (from GraphQL mutation and internal edges)
    // ========================================================================
    /// Admin requests to assess a website
    AssessWebsiteRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },

    /// Request to conduct Tavily searches (triggered by internal edge)
    ConductResearchSearchesRequested {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },

    /// Request to generate assessment from research (triggered by internal edge)
    GenerateAssessmentFromResearchRequested {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },

    // ========================================================================
    // Research Phase Events (Fact Events)
    // ========================================================================
    /// Research already exists and is recent enough to reuse
    WebsiteResearchFound {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        age_days: i64,
        requested_by: MemberId,
    },

    /// New research record created (homepage scraped and stored)
    WebsiteResearchCreated {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        homepage_url: String,
        requested_by: MemberId, // Added for internal edge to pass along
    },

    /// Failed to fetch or create research
    WebsiteResearchFailed {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    // ========================================================================
    // Search Phase Events (Fact Events)
    // ========================================================================
    /// All Tavily searches completed and results stored
    ResearchSearchesCompleted {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        total_queries: usize,
        total_results: usize,
        requested_by: MemberId, // Added for internal edge to pass along
    },

    /// Failed to conduct searches
    ResearchSearchesFailed {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },

    // ========================================================================
    // Assessment Phase Events (Fact Events)
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
