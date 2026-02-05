//! Discovery domain events
//!
//! ## PLATINUM RULE: Events Are Facts Only
//!
//! Events represent facts about what happened - never errors or failures.
//! Errors are returned via Result::Err, not as events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Discovery domain events - FACT EVENTS ONLY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryEvent {
    /// A discovery run completed (scheduled or manual)
    DiscoveryRunCompleted {
        run_id: Uuid,
        queries_executed: usize,
        total_results: usize,
        websites_created: usize,
        websites_filtered: usize,
    },
}
