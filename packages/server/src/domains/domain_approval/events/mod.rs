use crate::common::{JobId, MemberId, WebsiteId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DomainApprovalEvent {
    // ========================================================================
    // Request Event (from GraphQL mutation)
    // ========================================================================
    AssessWebsiteRequested {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },

    // ========================================================================
    // Research Phase Events
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
