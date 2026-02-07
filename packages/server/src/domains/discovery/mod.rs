//! Discovery domain - admin-managed search queries with AI pre-filtering
//!
//! Manages Tavily-based website discovery with:
//! - Configurable search queries (replaces hardcoded DISCOVERY_QUERIES)
//! - Plain-text AI filter rules (global and per-query)
//! - Discovery run tracking and query→website lineage
//!
//! Pipeline:
//!   Active queries (DB) → Tavily search → Dedup → AI pre-filter → Create website → Human approval
//!
//! Components:
//! - actions: run_discovery pipeline, AI filter evaluation
//! - models: DiscoveryQuery, DiscoveryFilterRule, DiscoveryRun, DiscoveryRunResult
//! - events: DiscoveryRunCompleted (terminal fact event)

pub mod activities;
pub mod effects;
pub mod events;
pub mod models;

// Re-export actions
pub use activities::{evaluate_websites_against_filters, run_discovery};

// Re-export events
pub use events::DiscoveryEvent;

// Re-export models
pub use models::{DiscoveryFilterRule, DiscoveryQuery, DiscoveryRun, DiscoveryRunResult};
