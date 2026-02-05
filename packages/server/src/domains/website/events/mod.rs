//! Website domain events
//!
//! Events are immutable facts that occurred during website approval and management workflows.
//!
//! Following the direct-call pattern:
//!   GraphQL → Action → emit(FactEvent) → Cascading Effects
//!
//! Note: *Requested events have been removed - GraphQL calls actions directly.

use serde::{Deserialize, Serialize};

use crate::common::{MemberId, WebsiteId};

/// Website domain events - fact events only (no request events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebsiteEvent {
    // =========================================================================
    // Fact Events (emitted by actions - what actually happened)
    // =========================================================================
    /// Website was approved for crawling
    WebsiteApproved {
        website_id: WebsiteId,
        reviewed_by: MemberId,
    },

    /// Website submission was rejected
    WebsiteRejected {
        website_id: WebsiteId,
        reason: String,
        reviewed_by: MemberId,
    },

    /// Website was suspended
    WebsiteSuspended {
        website_id: WebsiteId,
        reason: String,
        reviewed_by: MemberId,
    },

    /// Website crawl settings were updated
    CrawlSettingsUpdated {
        website_id: WebsiteId,
        max_pages_per_crawl: i32,
    },

    // =========================================================================
    // Authorization Events
    // =========================================================================
    /// User attempted admin action without permission
    AuthorizationDenied {
        user_id: MemberId,
        action: String,
        reason: String,
    },
}
