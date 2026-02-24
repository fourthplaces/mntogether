//! High-level extraction service wrapping the extraction library.
//!
//! This provides a simplified interface for server domains to use
//! the extraction library's capabilities.

use anyhow::{Context, Result};
use extraction::{
    ai::OpenAI as ExtractionOpenAI, types::page::CachedPage, DiscoverConfig, Extraction, Index,
    IngestResult, Ingestor, IngestorConfig, PostgresStore, QueryFilter, AI,
};
use sqlx::PgPool;
use std::sync::Arc;

/// High-level extraction service wrapping the extraction library.
///
/// Generic over the AI implementation to allow mocking in tests:
/// - Production: `ExtractionService<OpenAI>`
/// - Testing: `ExtractionService<MockAI>`
pub struct ExtractionService<A: AI> {
    index: Index<PostgresStore, A>,
}

impl<A: AI + Clone> ExtractionService<A> {
    /// Create a new extraction service.
    ///
    /// This will run extraction library migrations on the database.
    /// Use `from_store` if you have a pre-configured store.
    pub async fn from_pool_and_ai(pool: PgPool, ai: A) -> Result<Self> {
        let store = PostgresStore::from_pool(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create PostgresStore: {}", e))?;
        let index = Index::new(store, ai);
        Ok(Self { index })
    }

    /// Create from a pre-configured store.
    pub fn from_store(store: PostgresStore, ai: A) -> Self {
        let index = Index::new(store, ai);
        Self { index }
    }

    /// Get a reference to the underlying index.
    pub fn index(&self) -> &Index<PostgresStore, A> {
        &self.index
    }

    /// Search for pages and return unique site_urls.
    ///
    /// Used by semantic source search — finds which sites have content
    /// matching a query, without fetching full page content.
    pub async fn search_page_sites(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let page_refs = self
            .index
            .search(query, limit, None)
            .await
            .map_err(|e| anyhow::anyhow!("Search failed: {}", e))?;

        let mut seen = std::collections::HashSet::new();
        let site_urls: Vec<String> = page_refs
            .into_iter()
            .filter(|p| seen.insert(p.site_url.clone()))
            .map(|p| p.site_url)
            .collect();
        Ok(site_urls)
    }

    /// Search for relevant pages and return their raw content.
    ///
    /// Unlike `extract()` which returns RAG-summarized content, this returns
    /// the full original page content - better for structured post extraction.
    ///
    /// # Arguments
    /// * `query` - Natural language query (e.g., "volunteer opportunities")
    /// * `site` - Optional site filter (e.g., "redcross.org")
    /// * `limit` - Maximum number of pages to return
    pub async fn search_and_get_pages(
        &self,
        query: &str,
        site: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CachedPage>> {
        use extraction::traits::store::PageCache;

        let filter = self.build_site_filter(site);

        // Step 1: Search for relevant pages using embeddings
        let page_refs = self
            .index
            .search(query, limit, filter.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Search failed: {}", e))?;

        if page_refs.is_empty() {
            return Ok(vec![]);
        }

        // Step 2: Fetch full page content
        let urls: Vec<&str> = page_refs.iter().map(|p| p.url.as_str()).collect();
        let pages = self
            .index
            .store()
            .get_pages(&urls)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch pages: {}", e))?;

        Ok(pages)
    }

    /// Build a QueryFilter for site filtering.
    ///
    /// Handles all URL variants (http/https, www/non-www).
    fn build_site_filter(&self, site: Option<&str>) -> Option<QueryFilter> {
        site.map(|s| {
            let domain = s
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.");

            QueryFilter::for_sites([
                format!("https://{}", domain),
                format!("http://{}", domain),
                format!("https://www.{}", domain),
                format!("http://www.{}", domain),
            ])
        })
    }

    /// Extract information matching a query.
    ///
    /// Returns all extractions found. For queries that expect multiple items
    /// (collection queries), this may return many extractions.
    ///
    /// # Arguments
    /// * `query` - Natural language query (e.g., "volunteer opportunities")
    /// * `site` - Optional site filter (e.g., "redcross.org" or "https://redcross.org")
    ///
    /// Note: The site filter includes http/https and www/non-www variants,
    /// since extraction_pages.site_url may store URLs with any combination.
    pub async fn extract(&self, query: &str, site: Option<&str>) -> Result<Vec<Extraction>> {
        let filter = self.build_site_filter(site);
        self.index
            .extract(query, filter)
            .await
            .map_err(|e| anyhow::anyhow!("Extraction failed: {}", e))
    }

    /// Extract and return the first result, or an empty extraction if none found.
    ///
    /// Useful for singular queries where only one result is expected.
    pub async fn extract_one(&self, query: &str, site: Option<&str>) -> Result<Extraction> {
        let results = self.extract(query, site).await?;
        Ok(results
            .into_iter()
            .next()
            .unwrap_or_else(|| Extraction::new("No matching content found.".to_string())))
    }

    /// Extract with a pre-built filter.
    pub async fn extract_with_filter(
        &self,
        query: &str,
        filter: Option<QueryFilter>,
    ) -> Result<Vec<Extraction>> {
        self.index
            .extract(query, filter)
            .await
            .map_err(|e| anyhow::anyhow!("Extraction failed: {}", e))
    }

    // =========================================================================
    // Ingestion Methods
    // =========================================================================

    /// Ingest pages from a URL using the provided ingestor.
    ///
    /// This is the main entry point for adding content to the extraction index.
    /// The ingestor discovers pages, then Index processes them through the
    /// Summarize → Embed → Store pipeline.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use extraction::ingestors::{FirecrawlIngestor, ValidatedIngestor, DiscoverConfig};
    ///
    /// let ingestor = ValidatedIngestor::new(FirecrawlIngestor::from_env()?);
    /// let config = DiscoverConfig::new("https://example.com").with_limit(10);
    ///
    /// let result = extraction_service.ingest(&config, &ingestor).await?;
    /// println!("Ingested {} pages", result.pages_summarized);
    /// ```
    pub async fn ingest<I: Ingestor>(
        &self,
        discover_config: &DiscoverConfig,
        ingestor: &I,
    ) -> Result<IngestResult> {
        self.index
            .ingest(discover_config, ingestor)
            .await
            .map_err(|e| anyhow::anyhow!("Ingest failed: {}", e))
    }

    /// Ingest pages with custom configuration.
    ///
    /// Allows fine-tuning concurrency, caching behavior, and force resummarization.
    pub async fn ingest_with_config<I: Ingestor>(
        &self,
        discover_config: &DiscoverConfig,
        ingest_config: &IngestorConfig,
        ingestor: &I,
    ) -> Result<IngestResult> {
        self.index
            .ingest_with_config(discover_config, ingest_config, ingestor)
            .await
            .map_err(|e| anyhow::anyhow!("Ingest failed: {}", e))
    }

    /// Ingest specific URLs (for gap-filling or user submissions).
    ///
    /// Used by:
    /// - Detective gap-filling (fetch specific pages to answer questions)
    /// - User-submitted URLs
    /// - Re-fetching stale pages
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let urls = vec!["https://example.com/contact".to_string()];
    /// let result = extraction_service.ingest_urls(&urls, &ingestor).await?;
    /// ```
    pub async fn ingest_urls<I: Ingestor>(
        &self,
        urls: &[String],
        ingestor: &I,
    ) -> Result<IngestResult> {
        self.index
            .ingest_urls(urls, ingestor)
            .await
            .map_err(|e| anyhow::anyhow!("Ingest URLs failed: {}", e))
    }

    /// Ingest a single URL (convenience method).
    ///
    /// Wraps `ingest_urls` for the common single-URL case.
    pub async fn ingest_url<I: Ingestor>(&self, url: &str, ingestor: &I) -> Result<IngestResult> {
        self.ingest_urls(&[url.to_string()], ingestor).await
    }

    // =========================================================================
    // Direct Storage Methods
    // =========================================================================

    /// Store pages directly without summarization or embedding.
    ///
    /// Use this for pages that were fetched externally (e.g., via Tavily search)
    /// and need to be cached for later processing.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let pages = vec![
    ///     CachedPage::new(url, site_url, content).with_title(title),
    /// ];
    /// extraction_service.store_pages(&pages).await?;
    /// ```
    pub async fn store_pages(&self, pages: &[CachedPage]) -> Result<usize> {
        use extraction::traits::store::PageCache;

        let mut stored = 0;
        for page in pages {
            self.index
                .store()
                .store_page(page)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to store page {}: {}", page.url, e))?;
            stored += 1;
        }
        Ok(stored)
    }

    /// Store a single page directly.
    pub async fn store_page(&self, page: &CachedPage) -> Result<()> {
        use extraction::traits::store::PageCache;

        self.index
            .store()
            .store_page(page)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store page {}: {}", page.url, e))
    }

    /// Get all cached pages for a site_url.
    ///
    /// Used by org-level extraction to pool content from multiple sources.
    pub async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>> {
        use extraction::traits::store::PageCache;

        self.index
            .store()
            .get_pages_for_site(site_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get pages for site {}: {}", site_url, e))
    }
}

/// Type alias for OpenAI-backed extraction service.
pub type OpenAIExtractionService = ExtractionService<ExtractionOpenAI>;

/// Create an extraction service from environment configuration.
pub async fn create_extraction_service(pool: PgPool) -> Result<Arc<OpenAIExtractionService>> {
    let openai = ExtractionOpenAI::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;

    let service = ExtractionService::from_pool_and_ai(pool, openai)
        .await
        .context("Failed to create extraction service")?;

    Ok(Arc::new(service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_types_compile() {
        // Just verify the generic types work
        fn _assert_service<A: AI + Clone>(_service: &ExtractionService<A>) {}
    }
}
