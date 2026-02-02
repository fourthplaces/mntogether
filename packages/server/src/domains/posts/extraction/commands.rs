//! Post extraction commands
//!
//! Commands for the AI extraction workflow.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::common::{JobId, WebsiteId};

/// Post extraction commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostExtractionCommand {
    /// Extract posts from crawled page snapshots
    ExtractPostsFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        page_snapshot_ids: Vec<uuid::Uuid>,
    },
}

impl seesaw_core::Command for PostExtractionCommand {
    fn execution_mode(&self) -> seesaw_core::ExecutionMode {
        seesaw_core::ExecutionMode::Inline
    }

    fn job_spec(&self) -> Option<seesaw_core::JobSpec> {
        match self {
            Self::ExtractPostsFromPages { website_id, .. } => Some(seesaw_core::JobSpec {
                job_type: "extract_posts_from_pages",
                idempotency_key: Some(website_id.to_string()),
                max_retries: 2,
                priority: 0,
                version: 1,
            }),
        }
    }

    fn serialize_to_json(&self) -> Option<JsonValue> {
        serde_json::to_value(self).ok()
    }
}
