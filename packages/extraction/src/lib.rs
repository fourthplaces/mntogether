//! Domain-Agnostic Site Extraction Library
//!
//! A general-purpose, query-driven extraction library that can crawl any website
//! and extract structured information based on natural language queries.
//!
//! # Design Philosophy
//!
//! **"Let LLMs be LLMs"**
//!
//! - Query-driven, not schema-driven
//! - Plain text output (markdown), not rigid JSON
//! - Evidence-grounded for accuracy
//! - Maximum flexibility, minimum interference
//! - Library handles mechanics, app handles semantics
//!
//! # Usage
//!
//! ```rust,ignore
//! use extraction::{Index, QueryFilter, MemoryStore};
//! use extraction::testing::MockAI;
//!
//! // Initialize with storage backend
//! let store = MemoryStore::new();
//! let ai = MockAI::new();
//! let index = Index::new(store, ai);
//!
//! // Query ALL sites (no filter)
//! let all_opportunities = index.extract("volunteer opportunities", None).await?;
//!
//! // Query ONE site
//! let filter = QueryFilter::for_site("redcross.org");
//! let redcross_only = index.extract("volunteer opportunities", Some(filter)).await?;
//! ```
//!
//! # Modules
//!
//! - [`traits`] - Core trait abstractions (AI, PageStore, Crawler)
//! - [`types`] - Domain-agnostic data types
//! - [`pipeline`] - Extraction pipeline with strategy orchestration
//! - [`stores`] - Storage implementations (MemoryStore, etc.)
//! - [`crawlers`] - Crawler implementations (HttpCrawler, etc.)
//! - [`security`] - Credential handling and SSRF protection
//! - [`testing`] - Mock implementations for testing

pub mod crawlers;
pub mod error;
pub mod ingestors;
pub mod pipeline;
pub mod security;
pub mod stores;
pub mod testing;
pub mod traits;
pub mod types;

#[cfg(feature = "openai")]
pub mod ai;

// Re-export core types at crate root
pub use error::{CrawlError, ExtractionError, SecurityError};
pub use traits::{
    ai::AI,
    crawler::Crawler,
    ingestor::{DiscoverConfig, Ingestor, RawPage, ValidatedIngestor},
    searcher::{MockWebSearcher, SearchResult, TavilyWebSearcher, WebSearcher},
    store::{EmbeddingStore, PageCache, PageStore, SummaryCache},
};
pub use types::{
    config::{CrawlConfig, ExtractionConfig, QueryFilter},
    extraction::{
        Conflict, ConflictingClaim, Extraction, ExtractionStatus, GapQuery, GroundingGrade,
        MissingField, MissingReason, Source, SourceRole,
    },
    investigation::{
        GapType, InvestigationAction, InvestigationPlan, InvestigationStep, StepResult,
    },
    page::{CachedPage, PageRef},
    signals::{ExtractedSignal, StructuredSignals},
    summary::{RecallSignals, Summary},
};

// Re-export Index from pipeline
pub use pipeline::Index;

// Re-export pipeline components
pub use pipeline::{
    // Core functions
    classify_query, format_extract_prompt, format_partition_prompt, format_summarize_prompt,
    hybrid_recall, summarize_prompt_hash,
    // Grounding types
    Claim, ClaimGrounding, Evidence, GroundingConfig,
    // Strategy
    QueryAnalysis, RecallConfig,
    // Ingest (Crawler-based, legacy)
    ingest, ingest_single_page, refresh, IngestConfig, IngestResult, SinglePageResult,
    // Ingest (Ingestor-based, new)
    ingest_with_ingestor, ingest_urls_with_ingestor, IngestorConfig,
    // Extraction parsing
    parse_extraction_response, transform_extraction, transform_narrative_response,
    transform_single_response, AIExtractionResponse, AINarrativeResponse, AISingleResponse,
    ExtractionTransformConfig,
    // Partition
    default_partition, merge_similar_partitions, parse_partition_response, split_large_partition,
    validate_partitions,
};

// Re-export stores
pub use stores::MemoryStore;

#[cfg(feature = "sqlite")]
pub use stores::SqliteStore;

#[cfg(feature = "postgres")]
pub use stores::PostgresStore;

// Re-export crawlers
pub use crawlers::{
    // HTTP crawling
    HttpCrawler, RateLimitedCrawler, UrlValidator, ValidatedCrawler,
    // Informed crawling (query-driven)
    InformedCrawler, MockSearchService, SearchService, TavilySearchService,
    // Tavily search crawler
    QueryGenerator, TavilyCrawler,
    // robots.txt
    fetch_robots_txt, RobotsTxt,
};

// Re-export ingestors
pub use ingestors::{HttpIngestor, MockIngestor};

#[cfg(feature = "firecrawl")]
pub use ingestors::FirecrawlIngestor;

// Re-export testing utilities
pub use testing::{MockAI, MockCrawler, TestScenario};
