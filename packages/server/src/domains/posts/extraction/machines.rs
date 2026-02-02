//! Post extraction state machine
//!
//! This machine listens to CrawlEvent::PagesReadyForExtraction from the crawling domain
//! and triggers post extraction in the posts domain.
//!
//! This demonstrates cross-domain event routing: the posts domain reacts to crawling events.

use std::collections::HashSet;

use crate::common::WebsiteId;
use crate::domains::crawling::events::CrawlEvent;
use crate::domains::posts::extraction::commands::PostExtractionCommand;

/// Post extraction machine
///
/// Listens to CrawlEvent (cross-domain) and emits PostExtractionCommand
pub struct PostExtractionMachine {
    /// Track pending extractions to prevent duplicates
    pending_extractions: HashSet<WebsiteId>,
}

impl PostExtractionMachine {
    pub fn new() -> Self {
        Self {
            pending_extractions: HashSet::new(),
        }
    }

    fn cleanup_pending(&mut self, website_id: &WebsiteId) {
        self.pending_extractions.remove(website_id);
    }

    fn is_extraction_pending(&self, website_id: &WebsiteId) -> bool {
        self.pending_extractions.contains(website_id)
    }

    fn mark_extraction_pending(&mut self, website_id: WebsiteId) {
        self.pending_extractions.insert(website_id);
    }
}

impl Default for PostExtractionMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// This machine listens to CrawlEvent (from crawling domain) and produces PostExtractionCommand
impl seesaw_core::Machine for PostExtractionMachine {
    type Event = CrawlEvent;
    type Command = PostExtractionCommand;

    fn decide(&mut self, event: &CrawlEvent) -> Option<PostExtractionCommand> {
        match event {
            // =================================================================
            // CROSS-DOMAIN: Listen to crawling's integration event
            // =================================================================
            CrawlEvent::PagesReadyForExtraction {
                website_id,
                job_id,
                page_snapshot_ids,
            } => {
                if self.is_extraction_pending(website_id) {
                    return None;
                }
                self.mark_extraction_pending(*website_id);

                Some(PostExtractionCommand::ExtractPostsFromPages {
                    website_id: *website_id,
                    job_id: *job_id,
                    page_snapshot_ids: page_snapshot_ids.clone(),
                })
            }

            // Ignore other crawl events - we only care about PagesReadyForExtraction
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::JobId;
    use seesaw_core::Machine;

    #[test]
    fn test_listens_to_pages_ready() {
        let mut machine = PostExtractionMachine::new();
        let website_id = WebsiteId::new();
        let job_id = JobId::new();
        let page_snapshot_ids = vec![uuid::Uuid::new_v4()];

        let event = CrawlEvent::PagesReadyForExtraction {
            website_id,
            job_id,
            page_snapshot_ids: page_snapshot_ids.clone(),
        };

        let cmd = machine.decide(&event);
        assert!(matches!(
            cmd,
            Some(PostExtractionCommand::ExtractPostsFromPages { .. })
        ));
    }

    #[test]
    fn test_ignores_other_crawl_events() {
        let mut machine = PostExtractionMachine::new();
        let website_id = WebsiteId::new();
        let job_id = JobId::new();

        // WebsiteCrawled should be ignored - we only listen to PagesReadyForExtraction
        let event = CrawlEvent::WebsiteCrawled {
            website_id,
            job_id,
            pages: vec![],
        };

        assert!(machine.decide(&event).is_none());
    }

    #[test]
    fn test_prevents_duplicate_extractions() {
        let mut machine = PostExtractionMachine::new();
        let website_id = WebsiteId::new();
        let job_id = JobId::new();
        let page_snapshot_ids = vec![uuid::Uuid::new_v4()];

        let event = CrawlEvent::PagesReadyForExtraction {
            website_id,
            job_id,
            page_snapshot_ids: page_snapshot_ids.clone(),
        };

        // First call should trigger extraction
        assert!(machine.decide(&event).is_some());

        // Second call should be ignored (pending)
        assert!(machine.decide(&event).is_none());
    }
}
