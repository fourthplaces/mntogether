use async_trait::async_trait;
use url::Url;

use crate::new_types::*;

// ============================================================================
// STORAGE: Persistence (deterministic, replayable)
// ============================================================================

#[async_trait]
pub trait CrawlerStorage: Send + Sync {
    type ResourceId: Clone + Send + Sync;
    type PageId: Clone + Send + Sync;
    type ExtractionRunId: Clone + Send + Sync;
    type ExtractionId: Clone + Send + Sync;
    type Transaction: Send;
    type Error: std::error::Error + Send + Sync + 'static;

    // Transaction support
    async fn begin_transaction(&self) -> Result<Self::Transaction, Self::Error>;
    async fn commit_transaction(&self, tx: Self::Transaction) -> Result<(), Self::Error>;
    async fn rollback_transaction(&self, tx: Self::Transaction) -> Result<(), Self::Error>;

    // Resources
    async fn insert_resource(
        &self,
        resource: Resource,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<Self::ResourceId, Self::Error>;

    async fn get_resource(
        &self,
        id: Self::ResourceId,
    ) -> Result<Option<Resource>, Self::Error>;

    async fn update_resource_status(
        &self,
        id: Self::ResourceId,
        status: DiscoveryStatus,
        version: i32,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<(), Self::Error>;

    // Pages (canonical - one per URL)
    async fn upsert_page(
        &self,
        page: DiscoveredPage,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<UpsertResult<Self::PageId>, Self::Error>;

    async fn get_page(
        &self,
        id: Self::PageId,
    ) -> Result<Option<DiscoveredPage>, Self::Error>;

    async fn update_page_flag(
        &self,
        id: Self::PageId,
        flag: FlagResult,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<(), Self::Error>;

    async fn find_pages_to_refresh(
        &self,
        limit: usize,
    ) -> Result<Vec<PageToRefresh<Self::PageId>>, Self::Error>;

    // Discovery graph (M:N)
    async fn record_discovery_edge(
        &self,
        edge: ResourcePageEdge<Self::ResourceId, Self::PageId>,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<(), Self::Error>;

    // Extraction runs (audit trail)
    async fn create_extraction_run(
        &self,
        run: ExtractionRun<Self::PageId>,
    ) -> Result<Self::ExtractionRunId, Self::Error>;

    async fn finish_extraction_run(
        &self,
        run_id: Self::ExtractionRunId,
        stats: ExtractionStats,
    ) -> Result<(), Self::Error>;

    // Extractions (raw data)
    async fn insert_extraction(
        &self,
        extraction: RawExtraction,
        run_id: Self::ExtractionRunId,
    ) -> Result<Self::ExtractionId, Self::Error>;

    async fn get_extraction(
        &self,
        id: Self::ExtractionId,
    ) -> Result<Option<RawExtraction>, Self::Error>;

    async fn list_extractions_for_page(
        &self,
        page_id: Self::PageId,
    ) -> Result<Vec<RawExtraction>, Self::Error>;
}

// ============================================================================
// RATE LIMITER: Ephemeral coordination (separate from storage)
// ============================================================================

#[async_trait]
pub trait RateLimiter: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Attempt to acquire permission to fetch from domain
    async fn acquire(&self, domain: &str) -> Result<RateLimitDecision, Self::Error>;

    /// Release permission after fetch completes
    async fn release(&self, domain: &str) -> Result<(), Self::Error>;
}

// ============================================================================
// PAGE EVALUATOR: AI + heuristics (domain-agnostic)
// ============================================================================

#[async_trait]
pub trait PageEvaluator: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Cheap pre-filter before calling AI
    fn pre_filter(&self, url: &Url, content_snippet: &str) -> bool;

    /// Evaluate if page should be flagged
    async fn should_flag(
        &self,
        content: &PageContent,
    ) -> Result<FlagDecision, Self::Error>;

    /// Extract structured data (domain-agnostic - returns raw JSON)
    async fn extract_data(
        &self,
        content: &PageContent,
    ) -> Result<Vec<RawExtraction>, Self::Error>;
}

// ============================================================================
// PAGE FETCHER: Network access + link extraction
// ============================================================================

#[async_trait]
pub trait PageFetcher: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Fetch a single page
    async fn fetch(&self, url: &Url) -> Result<PageContent, Self::Error>;

    /// Extract links from a page
    async fn extract_links(
        &self,
        url: &Url,
        same_domain_only: bool,
    ) -> Result<Vec<Url>, Self::Error>;
}
