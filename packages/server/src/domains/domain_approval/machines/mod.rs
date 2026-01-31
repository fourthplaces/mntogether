use crate::common::MemberId;
use crate::domains::domain_approval::commands::DomainApprovalCommand;
use crate::domains::domain_approval::events::DomainApprovalEvent;
use seesaw_core::Machine;
use std::collections::HashMap;
use uuid::Uuid;

/// Website Approval Machine - Orchestrates the assessment workflow
///
/// Event Chain:
/// 1. AssessWebsiteRequested → FetchOrCreateResearch
/// 2. WebsiteResearchFound (recent) → GenerateAssessmentFromResearch
/// 3. WebsiteResearchCreated (new) → ConductResearchSearches
/// 4. ResearchSearchesCompleted → GenerateAssessmentFromResearch
/// 5. WebsiteAssessmentCompleted → [terminal]
pub struct DomainApprovalMachine {
    /// Track requesters for ongoing jobs (job_id → member_id)
    requesters: HashMap<Uuid, MemberId>,
}

impl DomainApprovalMachine {
    pub fn new() -> Self {
        Self {
            requesters: HashMap::new(),
        }
    }
}

impl Machine for DomainApprovalMachine {
    type Event = DomainApprovalEvent;
    type Command = DomainApprovalCommand;

    fn decide(&mut self, event: &DomainApprovalEvent) -> Option<DomainApprovalCommand> {
        match event {
            // Step 1: Admin requests assessment → fetch/create research
            DomainApprovalEvent::AssessWebsiteRequested {
                website_id,
                job_id,
                requested_by,
            } => {
                // Track requester for later steps
                self.requesters.insert(job_id.into_uuid(), *requested_by);

                Some(DomainApprovalCommand::FetchOrCreateResearch {
                    website_id: *website_id,
                    job_id: *job_id,
                    requested_by: *requested_by,
                })
            }

            // Step 2a: Research found and recent → skip to assessment generation
            DomainApprovalEvent::WebsiteResearchFound {
                research_id,
                website_id,
                job_id,
                requested_by,
                ..
            } => Some(DomainApprovalCommand::GenerateAssessmentFromResearch {
                research_id: *research_id,
                website_id: *website_id,
                job_id: *job_id,
                requested_by: *requested_by,
            }),

            // Step 2b: New research created → conduct Tavily searches
            DomainApprovalEvent::WebsiteResearchCreated {
                research_id,
                website_id,
                job_id,
                ..
            } => Some(DomainApprovalCommand::ConductResearchSearches {
                research_id: *research_id,
                website_id: *website_id,
                job_id: *job_id,
            }),

            // Step 3: Searches complete → generate assessment
            DomainApprovalEvent::ResearchSearchesCompleted {
                research_id,
                website_id,
                job_id,
                ..
            } => {
                // Retrieve requester from state
                let requested_by = self
                    .requesters
                    .get(&job_id.into_uuid())
                    .copied()
                    .unwrap_or_else(|| {
                        // Fallback: create a system user ID
                        MemberId::from_uuid(Uuid::nil())
                    });

                Some(DomainApprovalCommand::GenerateAssessmentFromResearch {
                    research_id: *research_id,
                    website_id: *website_id,
                    job_id: *job_id,
                    requested_by,
                })
            }

            // Terminal events - clean up state
            DomainApprovalEvent::WebsiteAssessmentCompleted { job_id, .. } => {
                self.requesters.remove(&job_id.into_uuid());
                None
            }

            // Failure events - clean up state
            DomainApprovalEvent::WebsiteResearchFailed { job_id, .. }
            | DomainApprovalEvent::ResearchSearchesFailed { job_id, .. }
            | DomainApprovalEvent::AssessmentGenerationFailed { job_id, .. } => {
                self.requesters.remove(&job_id.into_uuid());
                None
            }
        }
    }
}
