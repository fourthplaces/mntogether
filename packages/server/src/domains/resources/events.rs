//! Resource events - FACT EVENTS ONLY
//!
//! Events are immutable facts about what happened. Effects watch these
//! and can trigger cascades (e.g., ResourceDeleted → cleanup related data).

use crate::common::ResourceId;

/// Resource domain events - FACT EVENTS ONLY
#[derive(Debug, Clone)]
pub enum ResourceEvent {
    // =========================================================================
    // Fact Events (emitted by actions - what actually happened)
    // =========================================================================

    /// Resource was approved (status → Active)
    ResourceApproved { resource_id: ResourceId },

    /// Resource was rejected
    ResourceRejected {
        resource_id: ResourceId,
        reason: String,
    },

    /// Resource was edited
    ResourceEdited { resource_id: ResourceId },

    /// Resource was deleted - could trigger cascade cleanup
    ResourceDeleted { resource_id: ResourceId },
}
