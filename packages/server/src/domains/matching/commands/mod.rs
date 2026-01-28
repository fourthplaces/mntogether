use seesaw::{Command, ExecutionMode, JobSpec};
use uuid::Uuid;

/// Matching domain commands
#[derive(Debug, Clone)]
pub enum MatchingCommand {
    /// Find matching members for a need and send notifications
    FindMatches { need_id: Uuid },
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
            Self::FindMatches { need_id } => Some(JobSpec {
                job_type: "find_matches",
                idempotency_key: Some(need_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
                reference_id: Some(*need_id),
                container_id: None,
            }),
        }
    }

    fn serialize_to_json(&self) -> Option<serde_json::Value> {
        match self {
            Self::FindMatches { need_id } => Some(serde_json::json!({
                "need_id": need_id.to_string(),
            })),
        }
    }
}
