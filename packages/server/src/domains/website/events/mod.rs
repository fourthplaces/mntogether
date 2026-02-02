//! Website domain events
//!
//! Events are immutable facts that occurred during website approval and management workflows.

use crate::common::{MemberId, WebsiteId};

/// Website domain events
/// Following seesaw-rs pattern: Events are immutable facts
#[derive(Debug, Clone)]
pub enum WebsiteEvent {
    // =========================================================================
    // Request Events (from edges - entry points)
    // =========================================================================

    /// Admin requests to approve a website for crawling
    ApproveWebsiteRequested {
        website_id: WebsiteId,
        requested_by: MemberId,
    },

    /// Admin requests to reject a website submission
    RejectWebsiteRequested {
        website_id: WebsiteId,
        reason: String,
        requested_by: MemberId,
    },

    /// Admin requests to suspend an approved website
    SuspendWebsiteRequested {
        website_id: WebsiteId,
        reason: String,
        requested_by: MemberId,
    },

    /// Admin requests to update website crawl settings
    UpdateCrawlSettingsRequested {
        website_id: WebsiteId,
        max_pages_per_crawl: i32,
        requested_by: MemberId,
    },

    // =========================================================================
    // Fact Events (from effects - what actually happened)
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
