//! Domain Approval internal edges - event-to-event reactions
//!
//! Internal edges observe fact events and emit new request events.
//! This replaces the machine's decide() logic in seesaw 0.3.0.
//!
//! Event Chain:
//! 1. AssessWebsiteRequested → Effect → WebsiteResearchFound/WebsiteResearchCreated
//! 2. WebsiteResearchFound → InternalEdge → GenerateAssessmentFromResearchRequested
//! 3. WebsiteResearchCreated → InternalEdge → ConductResearchSearchesRequested
//! 4. ResearchSearchesCompleted → InternalEdge → GenerateAssessmentFromResearchRequested

use crate::domains::domain_approval::events::DomainApprovalEvent;

/// React to WebsiteResearchFound by triggering assessment generation.
///
/// When research is found and recent, skip to assessment generation.
pub fn on_research_found(event: &DomainApprovalEvent) -> Option<DomainApprovalEvent> {
    match event {
        DomainApprovalEvent::WebsiteResearchFound {
            research_id,
            website_id,
            job_id,
            requested_by,
            ..
        } => Some(DomainApprovalEvent::GenerateAssessmentFromResearchRequested {
            research_id: *research_id,
            website_id: *website_id,
            job_id: *job_id,
            requested_by: *requested_by,
        }),
        _ => None,
    }
}

/// React to WebsiteResearchCreated by triggering Tavily searches.
///
/// When new research is created, conduct searches to gather data.
pub fn on_research_created(event: &DomainApprovalEvent) -> Option<DomainApprovalEvent> {
    match event {
        DomainApprovalEvent::WebsiteResearchCreated {
            research_id,
            website_id,
            job_id,
            requested_by,
            ..
        } => Some(DomainApprovalEvent::ConductResearchSearchesRequested {
            research_id: *research_id,
            website_id: *website_id,
            job_id: *job_id,
            requested_by: *requested_by,
        }),
        _ => None,
    }
}

/// React to ResearchSearchesCompleted by triggering assessment generation.
///
/// When searches are complete, generate the AI assessment.
pub fn on_searches_completed(event: &DomainApprovalEvent) -> Option<DomainApprovalEvent> {
    match event {
        DomainApprovalEvent::ResearchSearchesCompleted {
            research_id,
            website_id,
            job_id,
            requested_by,
            ..
        } => Some(DomainApprovalEvent::GenerateAssessmentFromResearchRequested {
            research_id: *research_id,
            website_id: *website_id,
            job_id: *job_id,
            requested_by: *requested_by,
        }),
        _ => None,
    }
}

/// List of all domain approval internal edges.
///
/// The engine should call each of these when a DomainApprovalEvent fact is produced.
pub fn all_edges() -> Vec<fn(&DomainApprovalEvent) -> Option<DomainApprovalEvent>> {
    vec![on_research_found, on_research_created, on_searches_completed]
}
