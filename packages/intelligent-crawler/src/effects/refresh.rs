use uuid::Uuid;

use crate::{
    commands::CrawlerCommand,
    events::{CrawlerEvent, ScrapeStatus},
    new_types::*,
    traits::{CrawlerStorage, PageFetcher, RateLimiter},
};

/// Refresh effect handler (executes refresh commands)
pub struct RefreshEffect<S, F, R> {
    storage: S,
    fetcher: F,
    rate_limiter: R,
}

impl<S, F, R> RefreshEffect<S, F, R>
where
    S: CrawlerStorage<PageId = Uuid>, // ✅ Constrain to Uuid
    F: PageFetcher,
    R: RateLimiter,
{
    pub fn new(storage: S, fetcher: F, rate_limiter: R) -> Self {
        Self {
            storage,
            fetcher,
            rate_limiter,
        }
    }

    pub async fn execute(
        &self,
        cmd: CrawlerCommand,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        match cmd {
            CrawlerCommand::RefreshFlaggedPages { batch_size } => {
                self.refresh_flagged_pages(batch_size).await
            }
            CrawlerCommand::RefreshSpecificPage { page_id } => {
                self.refresh_specific_page(page_id).await
            }
            _ => Ok(vec![]),
        }
    }

    async fn refresh_flagged_pages(
        &self,
        batch_size: usize,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut events = Vec::new();
        let pages = self.storage.find_pages_to_refresh(batch_size).await?;

        let mut pages_checked = 0;
        let mut pages_changed = 0;

        for page_info in pages {
            // Check rate limit
            match self
                .rate_limiter
                .acquire(&page_info.url.domain().unwrap_or(""))
                .await?
            {
                RateLimitDecision::Allow => {}
                _ => continue, // Skip this page
            }

            // Fetch fresh content
            let fresh_content = match self.fetcher.fetch(&page_info.url).await {
                Ok(content) => content,
                Err(_) => {
                    self.rate_limiter
                        .release(&page_info.url.domain().unwrap_or(""))
                        .await?;

                    events.push(CrawlerEvent::PageContentChanged {
                        page_id: page_info.page_id,
                        old_content_hash: page_info.content_hash,
                        new_content_hash: String::new(),
                        scrape_status: ScrapeStatus::Failed,
                    });
                    continue;
                }
            };

            self.rate_limiter
                .release(&page_info.url.domain().unwrap_or(""))
                .await?;

            pages_checked += 1;

            // Check if content changed
            if fresh_content.content_hash != page_info.content_hash {
                pages_changed += 1;

                events.push(CrawlerEvent::PageContentChanged {
                    page_id: page_info.page_id,
                    old_content_hash: page_info.content_hash,
                    new_content_hash: fresh_content.content_hash,
                    scrape_status: ScrapeStatus::Ok,
                });
            } else {
                events.push(CrawlerEvent::PageContentUnchanged {
                    page_id: page_info.page_id,
                    content_hash: page_info.content_hash,
                });
            }
        }

        events.push(CrawlerEvent::RefreshCompleted {
            pages_checked,
            pages_changed,
        });

        Ok(events)
    }

    async fn refresh_specific_page(
        &self,
        page_id: Uuid,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let page = self
            .storage
            .get_page(page_id)
            .await?
            .ok_or("Page not found")?;

        // Check rate limit
        let domain = page.url.domain().unwrap_or("");
        match self.rate_limiter.acquire(domain).await? {
            RateLimitDecision::Allow => {}
            _ => return Err("Rate limited".into()),
        }

        // Fetch fresh content
        let fresh_content = match self.fetcher.fetch(&page.url).await {
            Ok(content) => content,
            Err(_e) => {
                self.rate_limiter.release(domain).await?;
                return Ok(vec![CrawlerEvent::PageContentChanged {
                    page_id,
                    old_content_hash: page.content_hash,
                    new_content_hash: String::new(),
                    scrape_status: ScrapeStatus::Failed,
                }]);
            }
        };

        self.rate_limiter.release(domain).await?;

        // Check if content changed
        if fresh_content.content_hash != page.content_hash {
            Ok(vec![CrawlerEvent::PageContentChanged {
                page_id,
                old_content_hash: page.content_hash,
                new_content_hash: fresh_content.content_hash,
                scrape_status: ScrapeStatus::Ok,
            }])
        } else {
            Ok(vec![CrawlerEvent::PageContentUnchanged {
                page_id,
                content_hash: page.content_hash,
            }])
        }
    }
}
