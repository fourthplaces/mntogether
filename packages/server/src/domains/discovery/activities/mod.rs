//! Discovery domain actions
//!
//! Business logic for the discovery pipeline:
//! - `run_discovery`: Main pipeline (query DB → Tavily search → AI filter → create websites)
//! - `evaluate_filter`: AI pre-filter evaluation against plain-text rules

pub mod evaluate_filter;
pub mod run_discovery;

pub use evaluate_filter::evaluate_websites_against_filters;
pub use run_discovery::run_discovery;
