//! Resources domain - extracted services/programs from websites
//!
//! This domain handles the simplified content model that replaces the complex
//! Listing model. Resources represent distinct services, programs, or opportunities
//! extracted from websites.
//!
//! Key features:
//! - Simpler normalized schema (no 20+ fields)
//! - AI semantic deduplication (embedding pre-filter + AI decision)
//! - Version history for audit trail
//! - Multiple source URLs per resource
//! - Direct tag associations

pub mod data;
pub mod edges;
pub mod effects;
pub mod models;

// Re-export models
pub use models::{
    ChangeReason, DedupDecision, Resource, ResourceSource, ResourceStatus, ResourceTag,
    ResourceVersion,
};

// Re-export data types (GraphQL types)
pub use data::{
    EditResourceInput, ResourceConnection, ResourceData, ResourceStatusData, ResourceVersionData,
};

// Re-export effects
pub use effects::{
    deduplicate_resource, sync_resources, DedupAction, ExtractedResourceInput, SyncResult,
};

// Re-export edges
pub use edges::{
    approve_resource, delete_resource, edit_and_approve_resource, edit_resource,
    get_active_resources, get_pending_resources, get_resource, get_resources, reject_resource,
};
