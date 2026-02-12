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

// Re-export AI client types
pub use ai_client::openai::StructuredOutput;
pub use ai_client::OpenAi;

/// GPT-5 Mini — cost-effective frontier model for extraction, dedup, sync, PII.
pub const GPT_5_MINI: &str = "gpt-5-mini";

/// GPT-5 — full frontier model for highest-accuracy tasks.
pub const GPT_5: &str = "gpt-5";

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
