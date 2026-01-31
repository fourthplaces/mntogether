use crate::common::{JobId, MemberId, WebsiteId};
use seesaw_core::{Command, ExecutionMode, JobSpec};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainApprovalCommand {
    /// Step 1: Fetch or create research data (homepage + metadata)
    FetchOrCreateResearch {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },

    /// Step 2: Conduct Tavily searches for the research
    ConductResearchSearches {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Step 3: Generate AI assessment from completed research
    GenerateAssessmentFromResearch {
        research_id: Uuid,
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
    },
}

impl Command for DomainApprovalCommand {
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Inline
    }

    fn job_spec(&self) -> Option<JobSpec> {
        match self {
            Self::FetchOrCreateResearch { website_id, .. } => Some(JobSpec {
                job_type: "fetch_website_research",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 2,
                priority: 5,
                version: 1,
            }),
            Self::ConductResearchSearches { research_id, .. } => Some(JobSpec {
                job_type: "conduct_research_searches",
                idempotency_key: Some(research_id.to_string()),
                max_retries: 2,
                priority: 5,
                version: 1,
            }),
            Self::GenerateAssessmentFromResearch { research_id, .. } => Some(JobSpec {
                job_type: "generate_assessment",
                idempotency_key: Some(research_id.to_string()),
                max_retries: 1,
                priority: 5,
                version: 1,
            }),
        }
    }
}
