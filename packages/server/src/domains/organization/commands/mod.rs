use serde_json::Value as JsonValue;
use std::net::IpAddr;

use crate::common::{JobId, MemberId, NeedId, PostId, SourceId};
use crate::domains::organization::events::ExtractedNeed;

/// Organization domain commands
/// Following seesaw-rs pattern: Commands are requests for IO operations
#[derive(Debug, Clone)]
pub enum OrganizationCommand {
    /// Scrape a source URL using Firecrawl
    ScrapeSource {
        source_id: SourceId,
        job_id: JobId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Extract needs from scraped content using AI
    ExtractNeeds {
        source_id: SourceId,
        job_id: JobId,
        organization_name: String,
        content: String,
    },

    /// Sync extracted needs with database
    SyncNeeds {
        source_id: SourceId,
        job_id: JobId,
        needs: Vec<ExtractedNeed>,
    },

    /// Create a new need (from user submission)
    CreateNeed {
        member_id: Option<MemberId>,
        organization_name: String,
        title: String,
        description: String,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        ip_address: Option<String>, // Converted from IpAddr before storing
        submission_type: String,    // 'user_submitted'
    },

    /// Update need status (for approval/rejection)
    UpdateNeedStatus {
        need_id: NeedId,
        status: String,
        rejection_reason: Option<String>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Update need content and approve it
    UpdateNeedAndApprove {
        need_id: NeedId,
        title: Option<String>,
        description: Option<String>,
        description_markdown: Option<String>,
        tldr: Option<String>,
        contact_info: Option<JsonValue>,
        urgency: Option<String>,
        location: Option<String>,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Create a post (when need is approved)
    CreatePost {
        need_id: NeedId,
        created_by: Option<MemberId>,
        custom_title: Option<String>,
        custom_description: Option<String>,
        expires_in_days: Option<i64>,
    },

    /// Generate embedding for a need (background job)
    GenerateNeedEmbedding { need_id: NeedId },

    /// Create a custom post (admin-created post with custom content)
    CreateCustomPost {
        need_id: NeedId,
        custom_title: Option<String>,
        custom_description: Option<String>,
        custom_tldr: Option<String>,
        targeting_hints: Option<JsonValue>,
        expires_in_days: Option<i64>,
        created_by: MemberId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Repost a need (create new post for existing active need)
    RepostNeed {
        need_id: NeedId,
        created_by: MemberId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Expire a post (mark as expired)
    ExpirePost {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Archive a post (mark as archived)
    ArchivePost {
        post_id: PostId,
        requested_by: MemberId,
        is_admin: bool,
    },

    /// Increment post view count (analytics)
    IncrementPostView { post_id: PostId },

    /// Increment post click count (analytics)
    IncrementPostClick { post_id: PostId },
}

// Implement Command trait for seesaw-rs integration
impl seesaw::Command for OrganizationCommand {
    fn execution_mode(&self) -> seesaw::ExecutionMode {
        use seesaw::ExecutionMode;

        match self {
            // Background commands - long-running IO operations
            Self::ScrapeSource { .. } => ExecutionMode::Background,
            Self::ExtractNeeds { .. } => ExecutionMode::Background,
            Self::SyncNeeds { .. } => ExecutionMode::Background,

            // Inline commands - fast database operations
            Self::CreateNeed { .. } => ExecutionMode::Inline,
            Self::UpdateNeedStatus { .. } => ExecutionMode::Inline,
            Self::UpdateNeedAndApprove { .. } => ExecutionMode::Inline,
            Self::CreatePost { .. } => ExecutionMode::Inline,
            Self::CreateCustomPost { .. } => ExecutionMode::Inline,
            Self::RepostNeed { .. } => ExecutionMode::Inline,
            Self::ExpirePost { .. } => ExecutionMode::Inline,
            Self::ArchivePost { .. } => ExecutionMode::Inline,
            Self::IncrementPostView { .. } => ExecutionMode::Inline,
            Self::IncrementPostClick { .. } => ExecutionMode::Inline,

            // Background - embedding generation
            Self::GenerateNeedEmbedding { .. } => ExecutionMode::Background,
        }
    }

    fn job_spec(&self) -> Option<seesaw::JobSpec> {
        match self {
            Self::ScrapeSource { source_id, .. } => Some(seesaw::JobSpec {
                job_type: "scrape_source",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
                reference_id: Some(*source_id.as_uuid()),
            }),
            Self::ExtractNeeds { source_id, .. } => Some(seesaw::JobSpec {
                job_type: "extract_needs",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
                reference_id: Some(*source_id.as_uuid()),
            }),
            Self::SyncNeeds { source_id, .. } => Some(seesaw::JobSpec {
                job_type: "sync_needs",
                idempotency_key: Some(source_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
                reference_id: Some(*source_id.as_uuid()),
            }),
            Self::GenerateNeedEmbedding { need_id } => Some(seesaw::JobSpec {
                job_type: "generate_need_embedding",
                idempotency_key: Some(need_id.to_string()),
                max_retries: 3,
                priority: 0,
                version: 1,
                reference_id: Some(*need_id.as_uuid()),
            }),
            // Inline commands don't need job specs
            _ => None,
        }
    }
}
