//! Facebook ingestor — wraps Apify scraping and outputs uniform RawPage objects.

use std::sync::Arc;

use async_trait::async_trait;
use extraction::error::{CrawlError, CrawlResult};
use extraction::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};
use tracing::info;

use apify_client::ApifyClient;

pub struct FacebookIngestor {
    apify: Arc<ApifyClient>,
}

impl FacebookIngestor {
    pub fn new(apify: Arc<ApifyClient>) -> Self {
        Self { apify }
    }
}

#[async_trait]
impl Ingestor for FacebookIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        let page_url = &config.url;
        let limit = config.limit.min(50) as u32;

        info!(page_url = %page_url, limit = limit, "Scraping Facebook posts via Apify");

        let posts = self
            .apify
            .scrape_facebook_posts(page_url, limit)
            .await
            .map_err(|e| CrawlError::Http(e.to_string().into()))?;

        let pages: Vec<RawPage> = posts
            .iter()
            .filter_map(|p| {
                let text = p.text.as_ref().filter(|t| !t.trim().is_empty())?;
                let post_url = p.url.as_ref()?;

                let mut content = format!("# Facebook Post\n\n{}\n\n---\n\n", text);
                if let Some(name) = &p.page_name {
                    content.push_str(&format!("**Author**: {}\n", name));
                }
                if let Some(ts) = &p.time {
                    content.push_str(&format!("**Posted**: {}\n", ts));
                }

                Some(
                    RawPage::new(post_url, content)
                        .with_title(
                            p.page_name
                                .clone()
                                .unwrap_or_else(|| "Facebook Post".to_string()),
                        )
                        .with_content_type("text/markdown".to_string())
                        .with_metadata("platform", "facebook"),
                )
            })
            .collect();

        info!(
            page_url = %page_url,
            total_scraped = posts.len(),
            pages_created = pages.len(),
            "Facebook ingest complete"
        );

        Ok(pages)
    }

    async fn fetch_specific(&self, _urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        Ok(vec![])
    }

    fn name(&self) -> &str {
        "facebook"
    }
}
