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

/// Ingest a single page on-the-fly.
///
/// This is the "expansion" capability for the discovery loop. When extraction
/// returns gaps, the app can use a `WebSearcher` to find new URLs, then call
/// this method to add them to the index without doing a full site crawl.
///
/// # Example
///
/// ```rust,ignore
/// // Extraction returned gaps
/// let result = index.extract("volunteer contact info").await?;
///
/// if result.needs_enrichment() {
///     // Use external search to find pages
///     let search_results = searcher.search(&result.gaps[0].query).await?;
///
///     // Ingest discovered pages
///     for search_result in search_results {
///         ingest_single_page(
///             search_result.url.as_str(),
///             &store,
///             &ai,
///             &crawler,
///         ).await?;
///     }
///
///     // Re-extract with enriched index
///     let enriched = index.extract("volunteer contact info").await?;
/// }
/// ```
pub async fn ingest_single_page<S, A, C>(
    url: &str,
    store: &S,
    ai: &A,
    crawler: &C,
) -> Result<SinglePageResult>
where
    S: PageStore,
    A: AI,
    C: Crawler,
{
    use crate::error::{CrawlError, ExtractionError};

    // Extract site URL from the page URL
    let parsed = url::Url::parse(url).map_err(|_| {
        ExtractionError::Crawl(CrawlError::InvalidUrl {
            url: url.to_string(),
        })
    })?;
    let site_url = format!(
        "{}://{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or("unknown")
    );

    // Check if page is already cached
    if let Ok(Some(existing)) = store.get_page(url).await {
        // Check if we have a fresh summary
        let current_prompt_hash = summarize_prompt_hash();
        if let Ok(Some(summary)) = store.get_summary(url, &existing.content_hash).await {
            if !summary.is_prompt_stale(&current_prompt_hash) {
                return Ok(SinglePageResult::AlreadyCached);
            }
        }
    }

    // Fetch the page
    let crawled = crawler
        .fetch(url)
        .await
        .map_err(ExtractionError::Crawl)?;

    // Convert to cached page
    let cached = CachedPage::new(&crawled.url, &site_url, &crawled.content)
        .with_title(crawled.title.unwrap_or_default());

    // Store the page
    store.store_page(&cached).await?;

    // Summarize and store
    let current_prompt_hash = summarize_prompt_hash();
    summarize_and_store(&cached, &site_url, &current_prompt_hash, store, ai).await?;

    Ok(SinglePageResult::Ingested {
        url: url.to_string(),
        site_url,
    })
}

/// Result of ingesting a single page.
#[derive(Debug, Clone)]
pub enum SinglePageResult {
    /// Page was already cached and fresh.
    AlreadyCached,

    /// Page was fetched, summarized, and stored.
    Ingested {
        /// The URL of the ingested page.
        url: String,
        /// The site URL derived from the page URL.
        site_url: String,
    },
}

impl SinglePageResult {
    /// Check if the page was newly ingested.
    pub fn was_ingested(&self) -> bool {
        matches!(self, Self::Ingested { .. })
    }
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

// =============================================================================
// Ingestor-based ingestion (new pattern)
// =============================================================================

/// Configuration for ingestor-based ingestion.
#[derive(Debug, Clone)]
pub struct IngestorConfig {
    /// Number of concurrent summarization tasks
    pub concurrency: usize,

    /// Skip pages that are already cached and fresh
    pub skip_cached: bool,

    /// Force re-summarization even if summary exists
    pub force_resummarize: bool,
}

impl Default for IngestorConfig {
    fn default() -> Self {
        Self {
            concurrency: 5,
            skip_cached: true,
            force_resummarize: false,
        }
    }
}

impl IngestorConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set concurrency.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Force re-summarization.
    pub fn force_resummarize(mut self) -> Self {
        self.force_resummarize = true;
        self
    }

    /// Don't skip cached pages.
    pub fn no_skip_cached(mut self) -> Self {
        self.skip_cached = false;
        self
    }
}

/// Ingest pages using an Ingestor.
///
/// This is the new preferred ingestion method that works with the pluggable
/// Ingestor trait instead of the Crawler trait.
///
/// # Example
///
/// ```rust,ignore
/// use extraction::ingestors::{FirecrawlIngestor, ValidatedIngestor, DiscoverConfig};
/// use extraction::pipeline::ingest::{ingest_with_ingestor, IngestorConfig};
///
/// let ingestor = ValidatedIngestor::new(FirecrawlIngestor::from_env()?);
/// let discover = DiscoverConfig::new("https://example.com").with_limit(10);
/// let config = IngestorConfig::default();
///
/// let result = ingest_with_ingestor(&discover, &config, &store, &ai, &ingestor).await?;
/// ```
pub async fn ingest_with_ingestor<S, A, I>(
    discover_config: &crate::traits::ingestor::DiscoverConfig,
    config: &IngestorConfig,
    store: &S,
    ai: &A,
    ingestor: &I,
) -> Result<IngestResult>
where
    S: PageStore,
    A: AI,
    I: crate::traits::ingestor::Ingestor,
{
    let mut result = IngestResult::new();

    // 1. Discover and fetch raw pages
    info!("Discovering pages from: {}", discover_config.url);
    let raw_pages = ingestor
        .discover(discover_config)
        .await
        .map_err(ExtractionError::Crawl)?;

    result.pages_crawled = raw_pages.len();
    info!("Discovered {} pages from {}", raw_pages.len(), discover_config.url);

    // 2. Convert to cached pages and store
    let mut pages_to_summarize: Vec<CachedPage> = Vec::new();
    let current_prompt_hash = summarize_prompt_hash();

    for raw in raw_pages {
        // Skip empty content
        if !raw.has_content() {
            continue;
        }

        // Extract site URL
        let site_url = raw.site_url().unwrap_or_else(|| discover_config.url.clone());

        // Create cached page
        let cached = CachedPage::new(&raw.url, &site_url, &raw.content)
            .with_fetched_at(raw.fetched_at);
        let cached = if let Some(title) = raw.title {
            cached.with_title(title)
        } else {
            cached
        };

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

    for page in pages_to_summarize {
        let _permit = semaphore.acquire().await.unwrap();
        let site_url = page.site_url.clone();

        match summarize_and_store(&page, &site_url, &current_prompt_hash, store, ai).await {
            Ok(_) => {
                result.pages_summarized += 1;
            }
            Err(e) => {
                warn!("Failed to summarize {}: {}", page.url, e);
                result.failed_urls.push(page.url.clone());
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

/// Ingest specific URLs using an Ingestor.
///
/// Used for:
/// - Detective gap-filling (fetch specific pages to answer questions)
/// - Re-fetching stale pages
/// - User-submitted URLs
pub async fn ingest_urls_with_ingestor<S, A, I>(
    urls: &[String],
    config: &IngestorConfig,
    store: &S,
    ai: &A,
    ingestor: &I,
) -> Result<IngestResult>
where
    S: PageStore,
    A: AI,
    I: crate::traits::ingestor::Ingestor,
{
    let mut result = IngestResult::new();

    // Fetch specific URLs
    let raw_pages = ingestor
        .fetch_specific(urls)
        .await
        .map_err(ExtractionError::Crawl)?;

    result.pages_crawled = raw_pages.len();
    let current_prompt_hash = summarize_prompt_hash();

    for raw in raw_pages {
        if !raw.has_content() {
            continue;
        }

        let site_url = raw.site_url().unwrap_or_else(|| raw.url.clone());

        let cached = CachedPage::new(&raw.url, &site_url, &raw.content)
            .with_fetched_at(raw.fetched_at);
        let cached = if let Some(title) = raw.title {
            cached.with_title(title)
        } else {
            cached
        };

        // Check cache
        if config.skip_cached && !config.force_resummarize {
            if let Ok(Some(existing_summary)) = store
                .get_summary(&cached.url, &cached.content_hash)
                .await
            {
                if !existing_summary.is_prompt_stale(&current_prompt_hash) {
                    result.pages_skipped += 1;
                    continue;
                }
            }
        }

        // Store page
        if let Err(e) = store.store_page(&cached).await {
            warn!("Failed to store page {}: {}", cached.url, e);
            result.failed_urls.push(cached.url.clone());
            continue;
        }

        // Summarize
        match summarize_and_store(&cached, &site_url, &current_prompt_hash, store, ai).await {
            Ok(_) => result.pages_summarized += 1,
            Err(e) => {
                warn!("Failed to summarize {}: {}", cached.url, e);
                result.failed_urls.push(cached.url.clone());
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

    #[tokio::test]
    async fn test_ingest_single_page() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new(
                "https://newsite.com/contact",
                "Contact page with email",
            ));

        let result = ingest_single_page(
            "https://newsite.com/contact",
            &store,
            &ai,
            &crawler,
        )
        .await
        .unwrap();

        assert!(result.was_ingested());
        if let SinglePageResult::Ingested { url, site_url } = result {
            assert_eq!(url, "https://newsite.com/contact");
            assert_eq!(site_url, "https://newsite.com");
        }

        // Verify stored
        assert_eq!(store.page_count(), 1);
        assert_eq!(store.summary_count(), 1);
    }

    #[tokio::test]
    async fn test_ingest_single_page_skips_cached() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let crawler = MockCrawler::new()
            .with_page(CrawledPage::new(
                "https://example.com/page",
                "Content",
            ));

        // First ingest
        let result1 = ingest_single_page(
            "https://example.com/page",
            &store,
            &ai,
            &crawler,
        )
        .await
        .unwrap();
        assert!(result1.was_ingested());

        // Second ingest - should be cached
        let result2 = ingest_single_page(
            "https://example.com/page",
            &store,
            &ai,
            &crawler,
        )
        .await
        .unwrap();
        assert!(!result2.was_ingested());
        assert!(matches!(result2, SinglePageResult::AlreadyCached));
    }

    #[tokio::test]
    async fn test_single_page_result_methods() {
        let cached = SinglePageResult::AlreadyCached;
        assert!(!cached.was_ingested());

        let ingested = SinglePageResult::Ingested {
            url: "https://example.com".to_string(),
            site_url: "https://example.com".to_string(),
        };
        assert!(ingested.was_ingested());
    }

    // ==========================================================================
    // Ingestor-based tests
    // ==========================================================================

    use crate::ingestors::{MockIngestor, RawPage, DiscoverConfig};

    #[tokio::test]
    async fn test_ingest_with_ingestor_basic() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let ingestor = MockIngestor::new();
        ingestor.add_page(RawPage::new("https://example.com/", "Home page content"));
        ingestor.add_page(RawPage::new("https://example.com/about", "About page content"));

        let discover = DiscoverConfig::new("https://example.com").with_limit(10);
        let config = IngestorConfig::default();

        let result = ingest_with_ingestor(&discover, &config, &store, &ai, &ingestor)
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
    async fn test_ingest_with_ingestor_skips_cached() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let ingestor = MockIngestor::new();
        ingestor.add_page(RawPage::new("https://example.com/", "Content"));

        let discover = DiscoverConfig::new("https://example.com").with_limit(10);
        let config = IngestorConfig::default();

        // First ingest
        let result1 = ingest_with_ingestor(&discover, &config, &store, &ai, &ingestor)
            .await
            .unwrap();
        assert_eq!(result1.pages_summarized, 1);

        // Second ingest - should skip
        let result2 = ingest_with_ingestor(&discover, &config, &store, &ai, &ingestor)
            .await
            .unwrap();
        assert_eq!(result2.pages_skipped, 1);
        assert_eq!(result2.pages_summarized, 0);
    }

    #[tokio::test]
    async fn test_ingest_urls_with_ingestor() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let ingestor = MockIngestor::new();
        ingestor.add_page(RawPage::new("https://example.com/a", "Page A"));
        ingestor.add_page(RawPage::new("https://example.com/b", "Page B"));

        let urls = vec![
            "https://example.com/a".to_string(),
            "https://example.com/b".to_string(),
        ];
        let config = IngestorConfig::default();

        let result = ingest_urls_with_ingestor(&urls, &config, &store, &ai, &ingestor)
            .await
            .unwrap();

        assert_eq!(result.pages_crawled, 2);
        assert_eq!(result.pages_summarized, 2);
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_ingest_with_ingestor_force_resummarize() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let ingestor = MockIngestor::new();
        ingestor.add_page(RawPage::new("https://example.com/", "Content"));

        let discover = DiscoverConfig::new("https://example.com").with_limit(10);

        // First ingest
        let config1 = IngestorConfig::default();
        ingest_with_ingestor(&discover, &config1, &store, &ai, &ingestor)
            .await
            .unwrap();

        // Second ingest with force
        let config2 = IngestorConfig::default().force_resummarize();
        let result = ingest_with_ingestor(&discover, &config2, &store, &ai, &ingestor)
            .await
            .unwrap();

        assert_eq!(result.pages_summarized, 1);
        assert_eq!(result.pages_skipped, 0);
    }
}
