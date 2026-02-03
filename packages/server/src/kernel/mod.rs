//! Kernel module - server infrastructure and dependencies.

pub mod ai;
pub mod ai_matching;
pub mod deps;
pub mod extraction_bridge;
pub mod extraction_service;
pub mod job_queue;
pub mod llm_request;
pub mod nats;
pub mod pii;
pub mod scheduled_tasks;
pub mod server_kernel;
pub mod tag;
pub mod test_dependencies;
pub mod traits;

// AI - primary export (backwards compatible wrapper)
pub use ai::OpenAIClient;

// Extraction library integration
pub use extraction_bridge::ExtractionAIBridge;
pub use extraction_service::{create_production_service, ExtractionService, ProductionExtractionService};

// Re-export from extraction library for easy access
pub use extraction::{
    // Ingestors
    DiscoverConfig, FirecrawlIngestor, HttpIngestor, Ingestor, IngestResult,
    MockIngestor, RawPage, ValidatedIngestor,
    // Web search
    MockWebSearcher, SearchResult, TavilyWebSearcher, WebSearcher,
};

// Other exports
pub use deps::{ServerDeps, TwilioAdapter};
pub use job_queue::{JobQueue, JobSpec};
pub use llm_request::LlmRequestExt;
pub use nats::{NatsClientPublisher, NatsPublisher, PublishedMessage, TestNats};
pub use pii::{create_pii_detector, HybridPiiDetector, NoopPiiDetector, RegexPiiDetector};
pub use server_kernel::ServerKernel;
pub use test_dependencies::{SpyJobQueue, TestDependencies};
pub use traits::*;
