use chrono::Duration;
use serde::{Deserialize, Serialize};
use seesaw::{Command, ExecutionMode};
use url::Url;
use uuid::Uuid;

/// Commands issued by Seesaw state machines (instructions for what to do)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlerCommand {
    // ============================================================================
    // Resource commands
    // ============================================================================
    SubmitResource {
        url: Url,
        submitted_by: Option<String>,
    },

    DiscoverResource {
        resource_id: Uuid,
        max_depth: usize,
        same_domain_only: bool,
    },

    ScheduleRediscovery {
        resource_id: Uuid,
        after: Duration,
    },

    // ============================================================================
    // Page commands
    // ============================================================================
    FlagPage {
        page_id: Uuid,
    },

    ExtractFromPage {
        page_id: Uuid,
    },

    RefreshSpecificPage {
        page_id: Uuid,
    },

    ScheduleRefresh {
        page_id: Uuid,
        after: Duration,
    },

    // ============================================================================
    // Flywheel commands
    // ============================================================================
    RefreshFlaggedPages {
        batch_size: usize,
    },
}

impl Command for CrawlerCommand {
    fn execution_mode(&self) -> ExecutionMode {
        // All crawler commands involve IO (network, database)
        ExecutionMode::Inline
    }
}
