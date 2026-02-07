//! Discovery domain - admin-managed search queries with AI pre-filtering

pub mod activities;
pub mod models;

// Re-export actions
pub use activities::{evaluate_websites_against_filters, run_discovery};

// Re-export models
pub use models::{DiscoveryFilterRule, DiscoveryQuery, DiscoveryRun, DiscoveryRunResult};
