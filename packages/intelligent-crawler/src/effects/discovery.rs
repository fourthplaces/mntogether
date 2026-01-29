use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    commands::CrawlerCommand,
    events::{CrawlerEvent, DiscoverySource},
    new_types::*,
    traits::{CrawlerStorage, PageEvaluator, PageFetcher, RateLimiter},
};

/// Discovery effect handler (executes DiscoverResource command)
pub struct DiscoveryEffect<S, F, E, R> {
    storage: S,
    fetcher: F,
    evaluator: E,
    rate_limiter: R,
}

impl<S, F, E, R> DiscoveryEffect<S, F, E, R>
where
    S: CrawlerStorage<ResourceId = Uuid, PageId = Uuid>, // ✅ Constrain to Uuid
    F: PageFetcher,
    E: PageEvaluator,
    R: RateLimiter,
{
    pub fn new(storage: S, fetcher: F, evaluator: E, rate_limiter: R) -> Self {
        Self {
            storage,
            fetcher,
            evaluator,
            rate_limiter,
        }
    }

    /// Execute DiscoverResource command → produce events
    pub async fn execute(
        &self,
        cmd: CrawlerCommand,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        match cmd {
            CrawlerCommand::DiscoverResource {
                resource_id,
                max_depth,
                same_domain_only,
            } => {
                self.discover_resource(resource_id, max_depth, same_domain_only)
                    .await
            }
            _ => Ok(vec![]), // Not our responsibility
        }
    }

    async fn discover_resource(
        &self,
        resource_id: Uuid,
        max_depth: usize,
        same_domain_only: bool,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut events = Vec::new();

        // Get resource
        let resource = self
            .storage
            .get_resource(resource_id)
            .await?
            .ok_or("Resource not found")?;

        let crawl_session_id = Uuid::new_v4();

        // Event: Discovery started
        events.push(CrawlerEvent::DiscoveryStarted {
            resource_id,
            crawl_session_id,
        });

        // Fetch the submitted URL itself (depth 0)
        match self
            .discover_page(
                &resource.url,
                resource_id,
                crawl_session_id,
                0,
                DiscoverySource::DirectSubmission,
            )
            .await
        {
            Ok(page_events) => events.extend(page_events),
            Err(e) => {
                events.push(CrawlerEvent::DiscoveryFailed {
                    resource_id,
                    crawl_session_id,
                    error: e.to_string(),
                });
                return Ok(events);
            }
        }

        // If max_depth > 0, crawl links
        if max_depth > 0 {
            let mut to_crawl = vec![(resource.url.clone(), 1)];
            let mut visited = HashSet::new();
            visited.insert(resource.url.to_string());

            while let Some((url, depth)) = to_crawl.pop() {
                if depth > max_depth {
                    continue;
                }

                // Extract links
                let links = match self.fetcher.extract_links(&url, same_domain_only).await {
                    Ok(links) => links,
                    Err(_) => continue, // Skip failed link extraction
                };

                for link in links {
                    let link_str = link.to_string();
                    if visited.contains(&link_str) {
                        continue;
                    }
                    visited.insert(link_str);

                    // Discover page
                    match self
                        .discover_page(
                            &link,
                            resource_id,
                            crawl_session_id,
                            depth as i32,
                            DiscoverySource::Crawl,
                        )
                        .await
                    {
                        Ok(page_events) => events.extend(page_events),
                        Err(_) => continue, // Skip failed pages
                    }

                    // Add to crawl queue
                    if depth < max_depth {
                        to_crawl.push((link, depth + 1));
                    }
                }
            }
        }

        // Event: Discovery completed (no counts - Seesaw will compute from events)
        events.push(CrawlerEvent::DiscoveryCompleted {
            resource_id,
            crawl_session_id,
        });

        Ok(events)
    }

    async fn discover_page(
        &self,
        url: &url::Url,
        resource_id: Uuid,
        crawl_session_id: Uuid,
        depth: i32,
        discovered_via: DiscoverySource,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut events = Vec::new();

        // Rate limit check
        let domain = url.domain().unwrap_or("");
        match self.rate_limiter.acquire(domain).await? {
            RateLimitDecision::Allow => {}
            _ => return Err("Rate limited".into()),
        }

        // Fetch content
        let content = self.fetcher.fetch(url).await?;
        self.rate_limiter.release(domain).await?;

        // Store page (idempotent upsert)
        let page = DiscoveredPage {
            url: url.clone(),
            domain: domain.to_string(),
            markdown: content.markdown.clone(),
            html: None,
            content_hash: content.content_hash.clone(),
            flag_status: FlagStatus::Pending,
            flagged_by: None,
            flag_confidence: None,
            flag_reason: None,
        };

        let upsert_result = self.storage.upsert_page(page, None).await?;

        // Record discovery edge
        let edge = ResourcePageEdge {
            resource_id,
            page_id: upsert_result.page_id,
            crawl_session_id,
            crawl_depth: depth,
            discovered_via,
        };
        self.storage.record_discovery_edge(edge, None).await?;

        // Event: Page discovered
        events.push(CrawlerEvent::PageDiscovered {
            resource_id,
            page_id: upsert_result.page_id,
            url: url.clone(),
            crawl_session_id,
            crawl_depth: depth,
            discovered_via,
        });

        Ok(events)
    }
}
