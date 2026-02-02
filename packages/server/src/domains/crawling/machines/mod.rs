//! Crawling domain state machines
//!
//! Pure decision logic - NO IO, only state transitions.
//! This machine coordinates the crawling workflow:
//!
//! - Crawling: Request → Crawl → Extract → Sync
//! - Regeneration: Regenerate posts/summaries from existing snapshots

use std::collections::HashSet;

use crate::common::WebsiteId;
use crate::domains::crawling::commands::CrawlCommand;
use crate::domains::crawling::events::CrawlEvent;

/// Crawling domain state machine
pub struct CrawlMachine {
    /// Track pending crawls to prevent duplicates
    pending_crawls: HashSet<WebsiteId>,
}

impl CrawlMachine {
    pub fn new() -> Self {
        Self {
            pending_crawls: HashSet::new(),
        }
    }

    /// Clean up pending state for a website
    fn cleanup_pending(&mut self, website_id: &WebsiteId) {
        self.pending_crawls.remove(website_id);
    }

    /// Check if a crawl is already pending for this website
    fn is_crawl_pending(&self, website_id: &WebsiteId) -> bool {
        self.pending_crawls.contains(website_id)
    }

    /// Mark a crawl as pending
    fn mark_crawl_pending(&mut self, website_id: WebsiteId) {
        self.pending_crawls.insert(website_id);
    }
}

impl Default for CrawlMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl seesaw_core::Machine for CrawlMachine {
    type Event = CrawlEvent;
    type Command = CrawlCommand;

    fn decide(&mut self, event: &CrawlEvent) -> Option<CrawlCommand> {
        match event {
            // =================================================================
            // CRAWLING WORKFLOW
            // =================================================================
            CrawlEvent::CrawlWebsiteRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                if self.is_crawl_pending(website_id) {
                    return None;
                }
                self.mark_crawl_pending(*website_id);
                Some(CrawlCommand::CrawlWebsite {
                    website_id: *website_id,
                    job_id: *job_id,
                    requested_by: *requested_by,
                    is_admin: *is_admin,
                })
            }

            // WebsiteCrawled is now informational only - extraction is handled by
            // PostExtractionMachine listening to PagesReadyForExtraction (cross-domain)
            CrawlEvent::WebsiteCrawled { .. } => None,

            CrawlEvent::PostsExtractedFromPages {
                website_id,
                job_id,
                posts,
                page_results,
            } => Some(CrawlCommand::SyncCrawledPosts {
                website_id: *website_id,
                job_id: *job_id,
                posts: posts.clone(),
                page_results: page_results.clone(),
            }),

            CrawlEvent::WebsiteCrawlNoListings {
                website_id,
                job_id,
                should_retry,
                ..
            } => {
                if *should_retry {
                    Some(CrawlCommand::RetryWebsiteCrawl {
                        website_id: *website_id,
                        job_id: *job_id,
                    })
                } else {
                    Some(CrawlCommand::MarkWebsiteNoPosts {
                        website_id: *website_id,
                        job_id: *job_id,
                    })
                }
            }

            // =================================================================
            // REGENERATION WORKFLOWS
            // =================================================================
            CrawlEvent::RegeneratePostsRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(CrawlCommand::RegeneratePosts {
                website_id: *website_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            CrawlEvent::RegeneratePageSummariesRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(CrawlCommand::RegeneratePageSummaries {
                website_id: *website_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            CrawlEvent::RegeneratePageSummaryRequested {
                page_snapshot_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(CrawlCommand::RegeneratePageSummary {
                page_snapshot_id: *page_snapshot_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            CrawlEvent::RegeneratePagePostsRequested {
                page_snapshot_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(CrawlCommand::RegeneratePagePosts {
                page_snapshot_id: *page_snapshot_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // =================================================================
            // TERMINAL EVENTS (no further action)
            // =================================================================
            CrawlEvent::PostsSynced { website_id, .. } => {
                self.cleanup_pending(website_id);
                None
            }

            CrawlEvent::WebsiteMarkedNoListings { website_id, .. }
            | CrawlEvent::WebsiteCrawlFailed { website_id, .. } => {
                self.cleanup_pending(website_id);
                None
            }

            // PagesReadyForExtraction is handled by PostExtractionMachine (cross-domain)
            CrawlEvent::PagesReadyForExtraction { .. }
            | CrawlEvent::PageSummariesRegenerated { .. }
            | CrawlEvent::PageSummaryRegenerated { .. }
            | CrawlEvent::PagePostsRegenerated { .. }
            | CrawlEvent::AuthorizationDenied { .. } => None,
        }
    }
}
