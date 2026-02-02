//! Website domain commands
//!
//! Commands are requests for IO operations in the website management workflow.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::common::{MemberId, WebsiteId};

/// Website domain commands
/// Following seesaw-rs pattern: Commands are requests for IO operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebsiteCommand {
    /// Approve a website for crawling
    ApproveWebsite {
        website_id: WebsiteId,
        requested_by: MemberId,
    },

    /// Reject a website submission
    RejectWebsite {
        website_id: WebsiteId,
        reason: String,
        requested_by: MemberId,
    },

    /// Suspend an approved website
    SuspendWebsite {
        website_id: WebsiteId,
        reason: String,
        requested_by: MemberId,
    },

    /// Update website crawl settings
    UpdateCrawlSettings {
        website_id: WebsiteId,
        max_pages_per_crawl: i32,
        requested_by: MemberId,
    },
}

// Implement Command trait for seesaw-rs integration
impl seesaw_core::Command for WebsiteCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        use seesaw_core::ExecutionMode;

        match self {
            // All commands run inline
            Self::ApproveWebsite { .. } => ExecutionMode::Inline,
            Self::RejectWebsite { .. } => ExecutionMode::Inline,
            Self::SuspendWebsite { .. } => ExecutionMode::Inline,
            Self::UpdateCrawlSettings { .. } => ExecutionMode::Inline,
        }
    }

    fn job_spec(&self) -> Option<seesaw_core::JobSpec> {
        // Inline commands don't need job specs
        None
    }

    fn serialize_to_json(&self) -> Option<JsonValue> {
        serde_json::to_value(self).ok()
    }
}
