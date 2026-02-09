//! Kernel module - server infrastructure and dependencies.

pub mod ai_tools;
pub mod deps;
pub mod extraction_service;
pub mod llm_request;
pub mod nats;
pub mod pii;
pub mod sse;
pub mod stream_hub;
pub mod tag;
pub mod test_dependencies;
pub mod traits;

// Re-export OpenAIClient from openai_client crate
pub use openai_client::OpenAIClient;

// Extraction library integration
pub use extraction_service::{
    create_extraction_service, ExtractionService, OpenAIExtractionService,
};

// Re-export from extraction library for easy access
pub use extraction::{
    // Ingestors
    DiscoverConfig,
    FirecrawlIngestor,
    HttpIngestor,
    IngestResult,
    Ingestor,
    MockIngestor,
    // Web search
    MockWebSearcher,
    RawPage,
    SearchResult,
    TavilyWebSearcher,
    ValidatedIngestor,
    WebSearcher,
};

// Other exports
pub use deps::{ServerDeps, TwilioAdapter};
pub use llm_request::CompletionExt;
pub use nats::{NatsClientPublisher, NatsPublisher, PublishedMessage, TestNats};
pub use pii::{create_pii_detector, HybridPiiDetector, NoopPiiDetector, RegexPiiDetector};
pub use stream_hub::StreamHub;
pub use test_dependencies::TestDependencies;
pub use traits::*;

// AI Tools for agentic workflows
pub use ai_tools::{FetchPageTool, SearchPostsTool, WebSearchTool};
