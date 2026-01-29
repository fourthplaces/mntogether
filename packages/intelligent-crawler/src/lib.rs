// New Seesaw-compatible architecture
pub mod events;
pub mod commands;
pub mod new_types;
pub mod traits;
pub mod effects;

// Re-exports for clean API
pub use events::{CrawlerEvent, AggregateKey, FlagSource, DiscoverySource, ScrapeStatus};
pub use commands::CrawlerCommand;
pub use new_types::*;
pub use traits::{CrawlerStorage, RateLimiter, PageEvaluator, PageFetcher};
pub use effects::{DiscoveryEffect, FlaggingEffect, ExtractionEffect, RefreshEffect};

// Old implementation (will be replaced/removed gradually)
pub mod types;
pub mod storage;
pub mod crawler;
pub mod detector;
pub mod extractor;
pub mod relationships;
pub mod config;

// Keep old exports for backward compatibility (temporary)
pub use storage::{Storage, PostgresStorage, PostgresCrawlerStorage};
pub use config::*;
pub use types::{PageSnapshotId, DetectionId, ExtractionId, SchemaId, RelationshipId, ContentHash};
