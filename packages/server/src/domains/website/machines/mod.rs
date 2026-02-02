//! Website domain state machine
//!
//! Pure decision logic - NO IO, only state transitions.
//! This machine coordinates the website approval and management workflow.

use crate::domains::website::commands::WebsiteCommand;
use crate::domains::website::events::WebsiteEvent;

/// Website domain state machine
pub struct WebsiteMachine;

impl WebsiteMachine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebsiteMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl seesaw_core::Machine for WebsiteMachine {
    type Event = WebsiteEvent;
    type Command = WebsiteCommand;

    fn decide(&mut self, event: &WebsiteEvent) -> Option<WebsiteCommand> {
        match event {
            // =================================================================
            // APPROVAL WORKFLOW
            // =================================================================
            WebsiteEvent::ApproveWebsiteRequested {
                website_id,
                requested_by,
            } => Some(WebsiteCommand::ApproveWebsite {
                website_id: *website_id,
                requested_by: *requested_by,
            }),

            WebsiteEvent::RejectWebsiteRequested {
                website_id,
                reason,
                requested_by,
            } => Some(WebsiteCommand::RejectWebsite {
                website_id: *website_id,
                reason: reason.clone(),
                requested_by: *requested_by,
            }),

            WebsiteEvent::SuspendWebsiteRequested {
                website_id,
                reason,
                requested_by,
            } => Some(WebsiteCommand::SuspendWebsite {
                website_id: *website_id,
                reason: reason.clone(),
                requested_by: *requested_by,
            }),

            // =================================================================
            // SETTINGS WORKFLOW
            // =================================================================
            WebsiteEvent::UpdateCrawlSettingsRequested {
                website_id,
                max_pages_per_crawl,
                requested_by,
            } => Some(WebsiteCommand::UpdateCrawlSettings {
                website_id: *website_id,
                max_pages_per_crawl: *max_pages_per_crawl,
                requested_by: *requested_by,
            }),

            // =================================================================
            // TERMINAL EVENTS (no further action)
            // =================================================================
            WebsiteEvent::WebsiteApproved { .. }
            | WebsiteEvent::WebsiteRejected { .. }
            | WebsiteEvent::WebsiteSuspended { .. }
            | WebsiteEvent::CrawlSettingsUpdated { .. }
            | WebsiteEvent::AuthorizationDenied { .. } => None,
        }
    }
}
