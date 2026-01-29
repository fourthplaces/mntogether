use seesaw_core::{Command, ExecutionMode, JobSpec};
use uuid::Uuid;

/// Member domain commands - intent for IO operations
#[derive(Debug, Clone)]
pub enum MemberCommand {
    RegisterMember {
        expo_push_token: String,
        searchable_text: String,
        city: String,
        state: String,
    },
    UpdateMemberStatus {
        member_id: Uuid,
        active: bool,
    },
    GenerateEmbedding {
        member_id: Uuid,
    },
}

impl Command for MemberCommand {
    fn execution_mode(&self) -> ExecutionMode {
        match self {
            // Inline - user waits for response
            Self::RegisterMember { .. } => ExecutionMode::Inline,
            Self::UpdateMemberStatus { .. } => ExecutionMode::Inline,
            // Background - generate embedding async
            Self::GenerateEmbedding { .. } => ExecutionMode::Background,
        }
    }

    fn job_spec(&self) -> Option<JobSpec> {
        match self {
            Self::GenerateEmbedding { member_id } => Some(JobSpec {
                job_type: "generate_member_embedding",
                idempotency_key: Some(member_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
                reference_id: Some(*member_id),
                container_id: None,
            }),
            _ => None,
        }
    }

    fn serialize_to_json(&self) -> Option<serde_json::Value> {
        match self {
            Self::GenerateEmbedding { member_id } => Some(serde_json::json!({
                "member_id": member_id.to_string(),
            })),
            _ => None,
        }
    }
}
