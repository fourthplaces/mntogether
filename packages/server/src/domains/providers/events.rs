//! Provider events - FACT EVENTS ONLY
//!
//! Events are immutable facts about what happened. Effects watch these
//! and call handlers directly for cascade workflows (no *Requested events).

use serde::{Deserialize, Serialize};

use crate::common::{MemberId, ProviderId};

/// Provider domain events - FACT EVENTS ONLY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderEvent {
    // =========================================================================
    // Fact Events (emitted by actions - what actually happened)
    // =========================================================================
    /// Provider was submitted (goes to pending_review)
    ProviderCreated {
        provider_id: ProviderId,
        name: String,
        submitted_by: Option<MemberId>,
    },

    /// Provider was approved
    ProviderApproved {
        provider_id: ProviderId,
        reviewed_by: MemberId,
    },

    /// Provider was rejected
    ProviderRejected {
        provider_id: ProviderId,
        reviewed_by: MemberId,
        reason: String,
    },

    /// Provider was suspended
    ProviderSuspended {
        provider_id: ProviderId,
        reviewed_by: MemberId,
        reason: String,
    },

    /// Provider was deleted - triggers cascade cleanup of contacts and tags
    ProviderDeleted { provider_id: ProviderId },
}
