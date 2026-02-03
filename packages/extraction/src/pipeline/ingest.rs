//! Ingestion pipeline - crawl, summarize, and index sites.

use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::error::{ExtractionError, Result};
use crate::pipeline::prompts::summarize_prompt_hash;
use crate::traits::{ai::AI, crawler::Crawler, store::PageStore};
use crate::types::{
    config::CrawlConfig,
    page::CachedPage,
    summary::Summary,
};

/// Result of an ingest operation.
#[derive(Debug, Clone)]
pub struct IngestResult {
    /// Number of pages crawled
    pub pages_crawled: usize,

    /// Number of pages successfully summarized
    pub pages_summarized: usize,

    /// Number of pages skipped (already cached and fresh)
    pub pages_skipped: usize,

    /// URLs that failed to process
    pub failed_urls: Vec<String>,
}

impl IngestResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            pages_crawled: 0,
            pages_summarized: 0,
            pages_skipped: 0,
            failed_urls: Vec::new(),
        }
    }

    /// Check if the ingest was fully successful.
    pub fn is_success(&self) -> bool {
        self.failed_urls.is_empty()
    }
}

impl Default for IngestResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for ingest operations.
#[derive(Debug, Clone)]
pub struct IngestConfig {
    /// Crawl configuration
    pub crawl: CrawlConfig,

    /// Number of concurrent summarization tasks
    pub concurrency: usize,

    /// Batch size for summarization (pages per LLM call)
    pub batch_size: usize,

    /// Skip pages that are already cached and fresh
    pub skip_cached: bool,

    /// Force re-summarization even if summary exists
    pub force_resummarize: bool,
}

impl IngestConfig {
    /// Create a new ingest config for a URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            crawl: CrawlConfig::new(url),
            concurrency: 5,
            batch_size: 5,
            skip_cached: true,
            force_resummarize: false,
        }
    }

    /// Set crawl config.
    pub fn with_crawl(mut self, crawl: CrawlConfig) -> Self {
        self.crawl = crawl;
        self
    }

    /// Set concurrency.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Force re-summarization.
    pub fn force_resummarize(mut self) -> Self {
        self.force_resummarize = true;
        self
    }
}

/// Ingest a site: crawl → summarize → store.
pub async fn ingest<S, A, C>(
    site_url: &str,
    config: &IngestConfig,
    store: &S,
    ai: &A,
    crawler: &C,
) -> Result<IngestResult>
where
    S: PageStore,
    A: AI,
    C: Crawler,
{
    let mut result = IngestResult::new();

    // 1. Crawl pages
    info!("Crawling site: {}", site_url);
    let crawled_pages = crawler
        .crawl(&config.crawl)
        .await
        .map_err(ExtractionError::Crawl)?;

    result.pages_crawled = crawled_pages.len();
    info!("Crawled {} pages from {}", crawled_pages.len(), site_url);

    // 2. Convert to cached pages and store
    let mut pages_to_summarize: Vec<CachedPage> = Vec::new();
    let current_prompt_hash = summarize_prompt_hash();

    for crawled in crawled_pages {
        let cached = CachedPage::new(&crawled.url, site_url, &crawled.content)
            .with_title(crawled.title.unwrap_or_default());

        // Check if we need to process this page
        if config.skip_cached && !config.force_resummarize {
            if let Ok(Some(existing_summary)) = store
                .get_summary(&cached.url, &cached.content_hash)
                .await
            {
                // Summary exists and content hasn't changed
                if !existing_summary.is_prompt_stale(&current_prompt_hash) {
                    result.pages_skipped += 1;
                    continue;
                }
            }
        }

        // Store the page
        if let Err(e) = store.store_page(&cached).await {
            warn!("Failed to store page {}: {}", cached.url, e);
            result.failed_urls.push(cached.url.clone());
            continue;
        }

        pages_to_summarize.push(cached);
    }

    info!(
        "Summarizing {} pages ({} skipped)",
        pages_to_summarize.len(),
        result.pages_skipped
    );

    // 3. Summarize pages (with concurrency control)
    let semaphore = Arc::new(Semaphore::new(config.concurrency));

    // Process in batches for efficiency
    for chunk in pages_to_summarize.chunks(config.batch_size) {
        let _permit = semaphore.acquire().await.unwrap();

        for page in chunk {
            match summarize_and_store(page, site_url, &current_prompt_hash, store, ai).await {
                Ok(_) => {
                    result.pages_summarized += 1;
                }
                Err(e) => {
                    warn!("Failed to summarize {}: {}", page.url, e);
                    result.failed_urls.push(page.url.clone());
                }
            }
        }
    }

    info!(
        "Ingest complete: {} crawled, {} summarized, {} skipped, {} failed",
        result.pages_crawled,
        result.pages_summarized,
        result.pages_skipped,
        result.failed_urls.len()
    );

    Ok(result)
}

/// Summarize a single page and store the result.
async fn summarize_and_store<S: PageStore, A: AI>(
    page: &CachedPage,
    site_url: &str,
    prompt_hash: &str,
    store: &S,
    ai: &A,
) -> Result<()> {
    // Get summary from AI
    let response = ai.summarize(&page.content, &page.url).await?;

    // Create summary with signals
    let mut summary = Summary::new(
        &page.url,
        site_url,
        &response.summary,
        &page.content_hash,
        prompt_hash,
    )
    .with_signals(response.signals);

    if let Some(lang) = response.language {
        summary = summary.with_language(lang);
    }

    // Generate and store embedding
    let embedding_text = summary.embedding_text();
    let embedding = ai.embed(&embedding_text).await?;
    summary = summary.with_embedding(embedding.clone());

    // Store summary
    store.store_summary(&summary).await?;

    // Store embedding separately (for search)
    store.store_embedding(&page.url, &embedding).await?;

    Ok(())
}

/// Refresh stale pages for a site.
pub async fn refresh<S, A, C>(
    site_url: &str,
    store: &S,
    ai: &A,
    crawler: &C,
) -> Result<IngestResult>
where
    S: PageStore,
    A: AI,
    C: Crawler,
{
    // Get existing pages for the site
    let existing_pages = store.get_pages_for_site(site_url).await?;

    let mut result = IngestResult::new();
    let current_prompt_hash = summarize_prompt_hash();

    for page in existing_pages {
        // Re-fetch the page
        match crawler.fetch(&page.url).await {
            Ok(fresh) => {
                let new_hash = CachedPage::hash_content(&fresh.content);

                // Check if content changed
                if new_hash != page.content_hash {
                    let cached = CachedPage::new(&fresh.url, site_url, &fresh.content)
                        .with_title(fresh.title.unwrap_or_default());

                    // Store updated page
                    store.store_page(&cached).await?;

                    // Re-summarize
                    match summarize_and_store(&cached, site_url, &current_prompt_hash, store, ai)
                        .await
                    {
                        Ok(_) => result.pages_summarized += 1,
                        Err(e) => {
                            warn!("Failed to summarize {}: {}", cached.url, e);
                            result.failed_urls.push(cached.url);
                        }
                    }
                } else {
                    result.pages_skipped += 1;
                }

                result.pages_crawled += 1;
            }
            Err(e) => {
                warn!("Failed to refresh {}: {}", page.url, e);
                result.failed_urls.push(page.url);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::MemoryStore;
    use crate::testing::{MockAI, MockCrawler};
    use crate::types::page::CrawledPage;

    #[tokio::test]
    async fn test_ingest_basic() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/", "Home page content"))
            .with_page(CrawledPage::new(
                "https://example.com/about",
                "About page content",
            ));

        let config = IngestConfig::new("https://example.com");
        let result = ingest("https://example.com", &config, &store, &ai, &crawler)
            .await
            .unwrap();

        assert_eq!(result.pages_crawled, 2);
        assert_eq!(result.pages_summarized, 2);
        assert!(result.is_success());

        // Verify stored
        assert_eq!(store.page_count(), 2);
        assert_eq!(store.summary_count(), 2);
    }

    #[tokio::test]
    async fn test_ingest_skips_cached() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/", "Content"));

        let config = IngestConfig::new("https://example.com");

        // First ingest
        let result1 = ingest("https://example.com", &config, &store, &ai, &crawler)
            .await
            .unwrap();
        assert_eq!(result1.pages_summarized, 1);

        // Second ingest - should skip
        let result2 = ingest("https://example.com", &config, &store, &ai, &crawler)
            .await
            .unwrap();
        assert_eq!(result2.pages_skipped, 1);
        assert_eq!(result2.pages_summarized, 0);
    }

    #[tokio::test]
    async fn test_ingest_force_resummarize() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new("https://example.com/", "Content"));

        // First ingest
        let config1 = IngestConfig::new("https://example.com");
        ingest("https://example.com", &config1, &store, &ai, &crawler)
            .await
            .unwrap();

        // Second ingest with force
        let config2 = IngestConfig::new("https://example.com").force_resummarize();
        let result = ingest("https://example.com", &config2, &store, &ai, &crawler)
            .await
            .unwrap();

        assert_eq!(result.pages_summarized, 1);
        assert_eq!(result.pages_skipped, 0);
    }
}
