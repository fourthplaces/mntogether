use intelligent_crawler::{CrawlerCommand, CrawlerEvent};
use seesaw::Machine;
use tracing::info;
use uuid::Uuid;

/// Page aggregate state (built from events)
#[derive(Debug, Clone)]
struct PageState {
    page_id: Uuid,
    flag_status: FlagStatus,
    content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlagStatus {
    Pending,
    Flagged,
    Unflagged,
    Error,
}

impl PageState {
    fn new(page_id: Uuid, content_hash: String) -> Self {
        Self {
            page_id,
            flag_status: FlagStatus::Pending,
            content_hash,
        }
    }

    /// Apply event to update state
    fn apply(&mut self, event: &CrawlerEvent) {
        match event {
            CrawlerEvent::PageFlagged { page_id, .. } => {
                if *page_id == self.page_id {
                    self.flag_status = FlagStatus::Flagged;
                }
            }
            CrawlerEvent::PageUnflagged { page_id, .. } => {
                if *page_id == self.page_id {
                    self.flag_status = FlagStatus::Unflagged;
                }
            }
            CrawlerEvent::PageFlaggingFailed { page_id, .. } => {
                if *page_id == self.page_id {
                    self.flag_status = FlagStatus::Error;
                }
            }
            CrawlerEvent::PageContentChanged { page_id, new_content_hash, .. } => {
                if *page_id == self.page_id {
                    self.content_hash = new_content_hash.clone();
                    // Content changed - need to re-flag
                    self.flag_status = FlagStatus::Pending;
                }
            }
            _ => {}
        }
    }
}

/// Page lifecycle state machine
/// Manages page flagging and extraction
pub struct PageLifecycleMachine {
    state: Option<PageState>,
}

impl PageLifecycleMachine {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl Machine for PageLifecycleMachine {
    type Event = CrawlerEvent;
    type Command = CrawlerCommand;

    fn decide(&mut self, event: &CrawlerEvent) -> Option<CrawlerCommand> {
        // Initialize or update state
        match event {
            CrawlerEvent::PageDiscovered { page_id, .. } => {
                info!(page_id = %page_id, "Page discovered, flagging");

                // Initialize state for new page (content_hash will be set when content changes)
                self.state = Some(PageState::new(*page_id, String::new()));

                // Start flagging
                return Some(CrawlerCommand::FlagPage { page_id: *page_id });
            }
            CrawlerEvent::PageContentChanged { page_id, new_content_hash, .. } => {
                info!(page_id = %page_id, "Page content changed, re-flagging");

                // Content changed - re-flag the page
                if let Some(ref mut state) = self.state {
                    state.apply(event);
                }

                return Some(CrawlerCommand::FlagPage { page_id: *page_id });
            }
            CrawlerEvent::PageFlagged { page_id, .. } => {
                info!(page_id = %page_id, "Page flagged, extracting");

                // Update state
                if let Some(ref mut state) = self.state {
                    state.apply(event);
                }

                // Page is flagged - extract data
                return Some(CrawlerCommand::ExtractFromPage { page_id: *page_id });
            }
            _ => {
                // Update existing state
                if let Some(ref mut state) = self.state {
                    state.apply(event);
                }
            }
        }

        None
    }
}

impl Default for PageLifecycleMachine {
    fn default() -> Self {
        Self::new()
    }
}
