//! Crawling domain internal edges - event-to-event reactions
//!
//! Internal edges observe fact events and emit new request events.
//! This replaces the machine's decide() logic in seesaw 0.3.0.
//!
//! Flow:
//!   Fact Event → Internal Edge → Option<Request Event>
//!
//! Event chains in crawling domain:
//! - PostsExtractedFromPages → SyncCrawledPostsRequested
//! - WebsiteCrawlNoListings → RetryWebsiteCrawlRequested or MarkWebsiteNoPostsRequested

use crate::domains::crawling::events::CrawlEvent;

/// React to PostsExtractedFromPages by triggering post sync.
///
/// When posts are extracted from crawled pages, we sync them to the database.
///
/// In the old machine architecture, this was:
/// ```ignore
/// CrawlEvent::PostsExtractedFromPages { .. } => {
///     Some(CrawlCommand::SyncCrawledPosts { .. })
/// }
/// ```
pub fn on_posts_extracted(event: &CrawlEvent) -> Option<CrawlEvent> {
    match event {
        CrawlEvent::PostsExtractedFromPages {
            website_id,
            job_id,
            posts,
            page_results,
        } => Some(CrawlEvent::SyncCrawledPostsRequested {
            website_id: *website_id,
            job_id: *job_id,
            posts: posts.clone(),
            page_results: page_results.clone(),
        }),
        _ => None,
    }
}

/// React to WebsiteCrawlNoListings by triggering retry or marking as no posts.
///
/// When a crawl finds no posts, we either retry or mark the website.
///
/// In the old machine architecture, this was:
/// ```ignore
/// CrawlEvent::WebsiteCrawlNoListings { should_retry, .. } => {
///     if should_retry { Some(CrawlCommand::RetryWebsiteCrawl { .. }) }
///     else { Some(CrawlCommand::MarkWebsiteNoPosts { .. }) }
/// }
/// ```
pub fn on_crawl_no_listings(event: &CrawlEvent) -> Option<CrawlEvent> {
    match event {
        CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            should_retry,
            ..
        } => {
            if *should_retry {
                Some(CrawlEvent::RetryWebsiteCrawlRequested {
                    website_id: *website_id,
                    job_id: *job_id,
                })
            } else {
                Some(CrawlEvent::MarkWebsiteNoPostsRequested {
                    website_id: *website_id,
                    job_id: *job_id,
                })
            }
        }
        _ => None,
    }
}

/// List of all crawling domain internal edges.
///
/// The engine should call each of these when a CrawlEvent fact is produced.
pub fn all_edges() -> Vec<fn(&CrawlEvent) -> Option<CrawlEvent>> {
    vec![on_posts_extracted, on_crawl_no_listings]
}
