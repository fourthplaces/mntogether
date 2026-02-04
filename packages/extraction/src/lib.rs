//! # Extraction Library
//!
//! A domain-agnostic, query-driven extraction library for crawling websites
//! and extracting structured information using LLMs.
//!
//! ## Design Philosophy
//!
//! **"Let LLMs be LLMs"**
//!
//! | Principle | Description |
//! |-----------|-------------|
//! | **Query-driven** | Natural language questions, not rigid schemas |
//! | **Domain-agnostic** | Signal types are user-defined strings |
//! | **Evidence-grounded** | Every claim cites sources; inference is flagged |
//! | **Mechanism vs Policy** | Library provides mechanics, caller controls behavior |
//!
//! ## Architecture
//!
//! ```text
//! INGEST → STORE → SUMMARIZE → EMBED → EXTRACT
//!
//! 1. Ingestor discovers/fetches pages → RawPage
//! 2. Pages stored in PageCache → CachedPage (with content hash)
//! 3. AI summarizes each page → Summary (with RecallSignals)
//! 4. AI embeds summaries → Vec<f32> stored in EmbeddingStore
//! 5. Query triggers: Recall → Partition → Extract → Extraction
//! ```
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use extraction::{Index, QueryFilter, MemoryStore, DiscoverConfig, ValidatedIngestor};
//! use extraction::ingestors::HttpIngestor;
//! use extraction::ai::OpenAI;
//!
//! // Setup
//! let store = MemoryStore::new();
//! let ai = OpenAI::from_env()?;
//! let index = Index::new(store, ai);
//!
//! // Ingest a website
//! let ingestor = ValidatedIngestor::new(HttpIngestor::new());
//! let config = DiscoverConfig::new("https://redcross.org").with_limit(50);
//! let result = index.ingest(&config, &ingestor).await?;
//!
//! // Extract (auto-selects Collection/Singular/Narrative strategy)
//! let extractions = index.extract("volunteer opportunities", None).await?;
//!
//! // Or filter to one site
//! let filter = QueryFilter::for_site("redcross.org");
//! let extractions = index.extract("volunteer opportunities", Some(filter)).await?;
//!
//! for extraction in extractions {
//!     println!("Content: {}", extraction.content);
//!     println!("Grounding: {:?}", extraction.grounding);
//!     println!("Gaps: {}", extraction.gaps.len());
//! }
//! ```
//!
//! ## Agent-Native Primitives
//!
//! For agents that want fine-grained control:
//!
//! ```rust,ignore
//! // Search the index (semantic + keyword hybrid)
//! let results = index.search("volunteer coordinator", 20, None).await?;
//!
//! // Read specific pages by URL
//! let pages = index.read(&["https://example.com/about"]).await?;
//!
//! // Extract from pages YOU select (skip recall phase)
//! let extraction = index.extract_from("contact information", &pages).await?;
//! ```
//!
//! ## Detective Engine (Gap Resolution)
//!
//! ```rust,ignore
//! let mut extraction = index.extract("board members", None).await?.remove(0);
//!
//! // Library provides intelligence (plan), caller provides will (iteration)
//! while extraction.has_gaps() && iterations < 3 {
//!     let plan = index.plan_investigation(&extraction);
//!     for step in &plan.steps {
//!         let result = index.execute_step(&step, None).await?;
//!         let pages = index.pages_from_step_result(&result).await?;
//!         let supplement = index.extract_from("board members", &pages).await?;
//!         extraction.merge(supplement);
//!     }
//!     iterations += 1;
//! }
//! ```
//!
//! ## Extraction Strategies
//!
//! | Strategy | Query Pattern | Behavior |
//! |----------|--------------|----------|
//! | **Collection** | "find all X" | Recall → Partition → Extract each bucket |
//! | **Singular** | "what is X?" | Recall top N → Extract single answer |
//! | **Narrative** | "describe X" | Recall top N → Aggregate narrative |
//!
//! ## Modules
//!
//! - [`traits`] - Core trait abstractions (AI, PageStore, Ingestor, Crawler)
//! - [`types`] - Domain-agnostic data types (Extraction, CachedPage, Summary)
//! - [`pipeline`] - Extraction pipeline with strategy orchestration
//! - [`stores`] - Storage implementations (MemoryStore, SqliteStore, PostgresStore)
//! - [`ingestors`] - Pluggable content fetching (HttpIngestor, FirecrawlIngestor)
//! - [`crawlers`] - Crawler implementations (HttpCrawler, InformedCrawler)
//! - [`security`] - SSRF protection and URL validation
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
    classify_query,
    // Partition
    default_partition,
    format_extract_prompt,
    format_partition_prompt,
    format_summarize_prompt,
    hybrid_recall,
    // Ingest
    ingest_urls_with_ingestor,
    ingest_with_ingestor,
    merge_similar_partitions,
    // Extraction parsing
    parse_extraction_response,
    parse_partition_response,
    split_large_partition,
    summarize_prompt_hash,
    transform_extraction,
    transform_narrative_response,
    transform_single_response,
    validate_partitions,
    AIExtractionResponse,
    AINarrativeResponse,
    AISingleResponse,
    // Grounding types
    Claim,
    ClaimGrounding,
    Evidence,
    ExtractionTransformConfig,
    GroundingConfig,
    IngestResult,
    IngestorConfig,
    // Strategy
    QueryAnalysis,
    RecallConfig,
};

// Re-export stores
pub use stores::MemoryStore;

#[cfg(feature = "sqlite")]
pub use stores::SqliteStore;

#[cfg(feature = "postgres")]
pub use stores::PostgresStore;

// Re-export crawlers
pub use crawlers::{
    // robots.txt
    fetch_robots_txt,
    // HTTP crawling
    HttpCrawler,
    // Informed crawling (query-driven)
    InformedCrawler,
    MockSearchService,
    // Tavily search crawler
    QueryGenerator,
    RateLimitedCrawler,
    RobotsTxt,
    SearchService,
    TavilyCrawler,
    TavilySearchService,
    UrlValidator,
    ValidatedCrawler,
};

// Re-export ingestors
pub use ingestors::{HttpIngestor, MockIngestor};

#[cfg(feature = "firecrawl")]
pub use ingestors::FirecrawlIngestor;

// Re-export testing utilities
pub use testing::{MockAI, MockCrawler, TestScenario};
