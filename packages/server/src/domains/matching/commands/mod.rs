use crate::common::ListingId;
use seesaw_core::{Command, ExecutionMode, JobSpec};

/// Matching domain commands
#[derive(Debug, Clone)]
pub enum MatchingCommand {
    /// Find matching members for a listing and send notifications
    FindMatches { listing_id: ListingId },
}

impl Command for MatchingCommand {
    fn execution_mode(&self) -> ExecutionMode {
        match self {
            // Background - matching can take time, don't block user
            Self::FindMatches { .. } => ExecutionMode::Background,
        }
    }

    fn job_spec(&self) -> Option<JobSpec> {
        match self {
            Self::FindMatches { listing_id } => Some(JobSpec {
                job_type: "find_matches",
                idempotency_key: Some(listing_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
                reference_id: Some(*listing_id.as_uuid()),
                container_id: None,
            }),
        }
    }

    fn serialize_to_json(&self) -> Option<serde_json::Value> {
        match self {
            Self::FindMatches { listing_id } => Some(serde_json::json!({
                "listing_id": listing_id.to_string(),
            })),
        }
    }
}
