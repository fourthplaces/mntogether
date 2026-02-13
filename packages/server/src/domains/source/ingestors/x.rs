//! X/Twitter ingestor — wraps Apify scraping and outputs uniform RawPage objects.

use std::sync::Arc;

use async_trait::async_trait;
use extraction::error::{CrawlError, CrawlResult};
use extraction::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};
use tracing::info;

use apify_client::ApifyClient;

pub struct XIngestor {
    apify: Arc<ApifyClient>,
}

impl XIngestor {
    pub fn new(apify: Arc<ApifyClient>) -> Self {
        Self { apify }
    }
}

#[async_trait]
impl Ingestor for XIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        let handle = config
            .options
            .get("handle")
            .ok_or_else(|| CrawlError::Http("X ingestor requires 'handle' option".into()))?;
        let limit = config.limit.min(50) as u32;

        info!(handle = %handle, limit = limit, "Scraping X/Twitter posts via Apify");

        let tweets = self
            .apify
            .scrape_x_posts(handle, limit)
            .await
            .map_err(|e| CrawlError::Http(e.to_string().into()))?;

        let pages: Vec<RawPage> = tweets
            .iter()
            .filter_map(|t| {
                let text = t.content().filter(|c| !c.trim().is_empty())?;
                let tweet_url = t.url.as_ref()?;

                let author_display = t
                    .author
                    .as_ref()
                    .and_then(|a| a.name.as_deref())
                    .unwrap_or(handle);

                let mut content = format!("# X Post\n\n{}\n\n---\n\n", text);
                content.push_str(&format!("**Author**: {}\n", author_display));
                if let Some(ts) = &t.created_at {
                    content.push_str(&format!("**Posted**: {}\n", ts));
                }

                Some(
                    RawPage::new(tweet_url, content)
                        .with_title(handle.to_string())
                        .with_content_type("text/markdown".to_string())
                        .with_metadata("platform", "x")
                        .with_metadata("handle", handle.to_string()),
                )
            })
            .collect();

        info!(
            handle = %handle,
            total_scraped = tweets.len(),
            pages_created = pages.len(),
            "X/Twitter ingest complete"
        );

        Ok(pages)
    }

    async fn fetch_specific(&self, _urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        Ok(vec![])
    }

    fn name(&self) -> &str {
        "x"
    }
}
