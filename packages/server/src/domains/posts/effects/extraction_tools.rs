//! Extraction tools data structures for agentic post enrichment
//!
//! These structures hold the collected data from tool calls during
//! the enrichment agent loop. The tool definitions themselves are
//! JSON-based (see `get_enrichment_tools()` in agentic_extraction.rs).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Import unified types from common
use crate::common::{CallToAction, ContactInfo, EligibilityInfo, LocationInfo, ScheduleInfo};

// =============================================================================
// Shared State for Tools
// =============================================================================

/// Collected enrichment data from tool calls.
/// Uses unified types from crate::common.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnrichmentData {
    pub contact: Option<ContactInfo>,
    pub location: Option<LocationInfo>,
    pub schedule: Option<ScheduleInfo>,
    pub eligibility: Option<EligibilityInfo>,
    pub call_to_action: Option<CallToAction>,
    pub finalized: bool,
    pub description: Option<String>,
    pub confidence: f32,
    pub notes: Vec<String>,
}

/// Thread-safe shared enrichment data for use across async tool calls
pub type SharedEnrichmentData = Arc<RwLock<EnrichmentData>>;
