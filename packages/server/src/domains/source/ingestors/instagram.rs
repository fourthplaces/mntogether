//! Instagram ingestor — wraps Apify scraping and outputs uniform RawPage objects.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use extraction::error::{CrawlError, CrawlResult};
use extraction::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};
use tracing::info;

use apify_client::ApifyClient;

pub struct InstagramIngestor {
    apify: Arc<ApifyClient>,
}

impl InstagramIngestor {
    pub fn new(apify: Arc<ApifyClient>) -> Self {
        Self { apify }
    }
}

#[async_trait]
impl Ingestor for InstagramIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        let handle = config.options.get("handle").ok_or_else(|| {
            CrawlError::Http("Instagram ingestor requires 'handle' option".into())
        })?;
        let limit = config.limit.min(50) as u32;

        info!(handle = %handle, limit = limit, "Scraping Instagram posts via Apify");

        let posts = self
            .apify
            .scrape_instagram_posts(handle, limit)
            .await
            .map_err(|e| CrawlError::Http(e.to_string().into()))?;

        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);

        let pages: Vec<RawPage> = posts
            .iter()
            .filter(|p| p.timestamp.map_or(true, |ts| ts >= thirty_days_ago))
            .filter_map(|p| {
                let caption = p.caption.as_ref().filter(|c| !c.trim().is_empty())?;

                let mut content = format!("# Instagram Post\n\n{}\n\n---\n\n", caption);
                if let Some(loc) = &p.location_name {
                    content.push_str(&format!("**Location**: {}\n", loc));
                }
                if let Some(ts) = p.timestamp {
                    content.push_str(&format!("**Posted**: {}\n", ts.format("%B %d, %Y")));
                }

                Some(
                    RawPage::new(&p.url, content)
                        .with_title(handle.to_string())
                        .with_content_type("text/markdown".to_string())
                        .with_fetched_at(p.timestamp.unwrap_or_else(Utc::now))
                        .with_metadata("platform", "instagram")
                        .with_metadata("handle", handle.to_string()),
                )
            })
            .collect();

        info!(
            handle = %handle,
            total_scraped = posts.len(),
            pages_created = pages.len(),
            "Instagram ingest complete"
        );

        Ok(pages)
    }

    async fn fetch_specific(&self, _urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        // Social platforms don't support fetching by URL via Apify
        Ok(vec![])
    }

    fn name(&self) -> &str {
        "instagram"
    }
}
