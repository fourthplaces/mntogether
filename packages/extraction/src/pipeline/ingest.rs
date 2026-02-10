//! Ingestion pipeline - crawl, summarize, and index sites.
//!
//! This module provides the ingestion pipeline that processes pages through:
//! 1. Discovery - via pluggable Ingestors (HttpIngestor, FirecrawlIngestor, etc.)
//! 2. Summarization - via AI
//! 3. Embedding - via AI
//! 4. Storage - via PageStore

use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

use crate::error::{ExtractionError, Result};
use crate::pipeline::prompts::summarize_prompt_hash;
use crate::traits::ingestor::RawPage;
use crate::traits::{ai::AI, store::PageStore};
use crate::types::{page::CachedPage, summary::Summary};

/// Minimum content length (in chars) for a page to be worth summarizing.
/// Pages shorter than this are likely non-content pages (cart, login, etc.)
const MIN_CONTENT_LENGTH_FOR_SUMMARY: usize = 50;

// =============================================================================
// URL Normalization
// =============================================================================

/// Normalize a URL to a canonical form for deduplication.
///
/// - Ensures https scheme
/// - Removes www. prefix (canonical: non-www)
/// - Removes trailing slashes from path
/// - Removes query parameters (they often represent modal states, not content)
/// - Removes fragments
fn normalize_url(url: &str) -> String {
    // Parse the URL
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return url.to_string(), // Return as-is if unparseable
    };

    // Get host, removing www. prefix
    let host = parsed.host_str().unwrap_or("").trim_start_matches("www.");

    // Get path, removing trailing slash (but keep "/" for root)
    let path = parsed.path().trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };

    // Reconstruct without query params or fragments
    format!("https://{}{}", host, path)
}

/// Normalize a site URL (just the origin).
fn normalize_site_url(url: &str) -> String {
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return url.to_string(),
    };

    let host = parsed.host_str().unwrap_or("").trim_start_matches("www.");

    format!("https://{}", host)
}

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

// =============================================================================
// Shared helpers
// =============================================================================

/// Check if a page should be skipped (already cached with fresh summary).
async fn should_skip_page<S: PageStore>(
    url: &str,
    content_hash: &str,
    prompt_hash: &str,
    skip_cached: bool,
    force_resummarize: bool,
    store: &S,
) -> bool {
    if !skip_cached || force_resummarize {
        return false;
    }

    if let Ok(Some(existing_summary)) = store.get_summary(url, content_hash).await {
        if !existing_summary.is_prompt_stale(prompt_hash) {
            return true;
        }
    }
    false
}

/// Convert a RawPage to CachedPage with normalized URLs.
fn raw_to_cached(raw: &RawPage, fallback_site_url: &str) -> CachedPage {
    let site_url = raw
        .site_url()
        .unwrap_or_else(|| fallback_site_url.to_string());

    // Normalize URLs for deduplication
    let normalized_url = normalize_url(&raw.url);
    let normalized_site_url = normalize_site_url(&site_url);

    let cached = CachedPage::new(&normalized_url, &normalized_site_url, &raw.content)
        .with_fetched_at(raw.fetched_at);
    if let Some(ref title) = raw.title {
        cached.with_title(title.clone())
    } else {
        cached
    }
}

/// Process a list of pages: check cache, store, and summarize.
/// Returns the number of pages summarized and skipped.
async fn process_pages<S: PageStore, A: AI>(
    pages: Vec<CachedPage>,
    prompt_hash: &str,
    skip_cached: bool,
    force_resummarize: bool,
    concurrency: usize,
    store: &S,
    ai: &A,
    result: &mut IngestResult,
) {
    let mut pages_to_summarize: Vec<CachedPage> = Vec::new();

    for cached in pages {
        // Check cache
        if should_skip_page(
            &cached.url,
            &cached.content_hash,
            prompt_hash,
            skip_cached,
            force_resummarize,
            store,
        )
        .await
        {
            result.pages_skipped += 1;
            continue;
        }

        // Store the page
        if let Err(e) = store.store_page(&cached).await {
            warn!("Failed to store page {}: {}", cached.url, e);
            result.failed_urls.push(cached.url.clone());
            continue;
        }

        pages_to_summarize.push(cached);
    }

    if !pages_to_summarize.is_empty() {
        debug!(
            to_summarize = pages_to_summarize.len(),
            skipped = result.pages_skipped,
            "Processing pages"
        );
    }

    // Summarize with concurrency control
    let semaphore = Arc::new(Semaphore::new(concurrency));

    for page in pages_to_summarize {
        let _permit = semaphore.acquire().await.unwrap();
        let site_url = page.site_url.clone();

        match summarize_and_store(&page, &site_url, prompt_hash, store, ai).await {
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

// =============================================================================
// Ingestor-based ingestion
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
/// This is the main ingestion method that works with pluggable Ingestors.
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
    let raw_pages = ingestor
        .discover(discover_config)
        .await
        .map_err(ExtractionError::Crawl)?;

    result.pages_crawled = raw_pages.len();
    debug!(
        url = %discover_config.url,
        pages = raw_pages.len(),
        "Discovered pages"
    );

    // 2. Convert to cached pages (skip empty/short content)
    let pages: Vec<CachedPage> = raw_pages
        .iter()
        .filter(|raw| {
            if !raw.has_content() {
                return false;
            }
            if raw.content.trim().len() < MIN_CONTENT_LENGTH_FOR_SUMMARY {
                debug!(url = %raw.url, len = raw.content.trim().len(), "Skipping page with insufficient content");
                return false;
            }
            true
        })
        .map(|raw| raw_to_cached(raw, &discover_config.url))
        .collect();

    let content_filtered = raw_pages.len() - pages.len();

    // 3. Process pages (cache check, store, summarize)
    let prompt_hash = summarize_prompt_hash();
    process_pages(
        pages,
        &prompt_hash,
        config.skip_cached,
        config.force_resummarize,
        config.concurrency,
        store,
        ai,
        &mut result,
    )
    .await;

    result.pages_skipped += content_filtered;

    info!(
        url = %discover_config.url,
        crawled = result.pages_crawled,
        summarized = result.pages_summarized,
        skipped = result.pages_skipped,
        failed = result.failed_urls.len(),
        "Ingest complete"
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

    // 1. Fetch specific URLs
    let raw_pages = ingestor
        .fetch_specific(urls)
        .await
        .map_err(ExtractionError::Crawl)?;

    result.pages_crawled = raw_pages.len();

    // 2. Convert to cached pages (skip empty/short content, use URL as fallback site)
    let pages: Vec<CachedPage> = raw_pages
        .iter()
        .filter(|raw| {
            if !raw.has_content() {
                return false;
            }
            if raw.content.trim().len() < MIN_CONTENT_LENGTH_FOR_SUMMARY {
                debug!(url = %raw.url, len = raw.content.trim().len(), "Skipping page with insufficient content");
                return false;
            }
            true
        })
        .map(|raw| raw_to_cached(raw, &raw.url))
        .collect();

    let content_filtered = raw_pages.len() - pages.len();

    // 3. Process pages (cache check, store, summarize)
    let prompt_hash = summarize_prompt_hash();
    process_pages(
        pages,
        &prompt_hash,
        config.skip_cached,
        config.force_resummarize,
        config.concurrency,
        store,
        ai,
        &mut result,
    )
    .await;

    result.pages_skipped += content_filtered;

    info!(
        urls = urls.len(),
        summarized = result.pages_summarized,
        skipped = result.pages_skipped,
        "URL ingest complete"
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestors::{DiscoverConfig, MockIngestor, RawPage};
    use crate::stores::MemoryStore;
    use crate::testing::MockAI;

    #[test]
    fn test_normalize_url() {
        // Basic normalization
        assert_eq!(
            normalize_url("https://example.com/page"),
            "https://example.com/page"
        );

        // www removal
        assert_eq!(
            normalize_url("https://www.example.com/page"),
            "https://example.com/page"
        );

        // http -> https
        assert_eq!(
            normalize_url("http://example.com/page"),
            "https://example.com/page"
        );

        // Trailing slash removal
        assert_eq!(
            normalize_url("https://example.com/page/"),
            "https://example.com/page"
        );

        // Root path keeps slash
        assert_eq!(
            normalize_url("https://example.com/"),
            "https://example.com/"
        );
        assert_eq!(normalize_url("https://example.com"), "https://example.com/");

        // Query params removed
        assert_eq!(
            normalize_url("https://example.com/page?overlay=modal&id=123"),
            "https://example.com/page"
        );

        // Fragment removed
        assert_eq!(
            normalize_url("https://example.com/page#section"),
            "https://example.com/page"
        );

        // Combined
        assert_eq!(
            normalize_url("http://www.example.com/page/?query=1#frag"),
            "https://example.com/page"
        );
    }

    #[test]
    fn test_normalize_site_url() {
        assert_eq!(
            normalize_site_url("https://www.example.com/page"),
            "https://example.com"
        );
        assert_eq!(
            normalize_site_url("http://example.com"),
            "https://example.com"
        );
    }

    #[tokio::test]
    async fn test_ingest_with_ingestor_basic() {
        let store = MemoryStore::new();
        let ai = MockAI::new();
        let ingestor = MockIngestor::new();
        ingestor.add_page(RawPage::new("https://example.com/", "Home page content with enough text to pass the minimum content length filter for summarization"));
        ingestor.add_page(RawPage::new(
            "https://example.com/about",
            "About page content with enough text to pass the minimum content length filter for summarization",
        ));

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
        ingestor.add_page(RawPage::new("https://example.com/", "Content with enough text to pass the minimum content length filter for summarization"));

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
        ingestor.add_page(RawPage::new("https://example.com/a", "Page A content with enough text to pass the minimum content length filter for summarization"));
        ingestor.add_page(RawPage::new("https://example.com/b", "Page B content with enough text to pass the minimum content length filter for summarization"));

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
        ingestor.add_page(RawPage::new("https://example.com/", "Content with enough text to pass the minimum content length filter for summarization"));

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
