// Effects (side effects) for posts domain
//
// Effects are thin orchestrators that delegate to domain functions.
// Domain logic lives in separate function modules.
//
// NOTE: Crawler effect has been moved to the `crawling` domain.
// See `crate::domains::crawling::effects::CrawlerEffect`.

pub mod ai;
pub mod composite;
pub mod deduplication; // LLM-based post deduplication
pub mod discovery; // Static search queries for finding community resources
pub mod extraction; // Two-pass extraction: summarize pages, then synthesize posts (DEPRECATED: use crawling::effects::extraction)
pub mod post;
pub mod post_extraction; // Domain functions for AI extraction
pub mod post_operations; // Domain functions for post CRUD operations
pub mod post_report;
pub mod scraper;
pub mod sync;
pub mod syncing; // Domain functions for post synchronization
pub mod utils;

pub use ai::*;
pub use composite::*;
pub use discovery::{run_discovery_searches, DiscoveryResult, DISCOVERY_QUERIES, DEFAULT_LOCATION};
pub use post::*;
pub use scraper::*;
pub use sync::*;
pub use utils::*;

// Re-export ServerDeps from kernel for backwards compatibility
pub use crate::kernel::ServerDeps;

// Domain function modules are available via:
//   - `effects::post_extraction::*` - AI extraction functions
//   - `effects::post_operations::*` - Post CRUD operations
//   - `effects::syncing::*` - Post synchronization functions
// (not re-exported at top level to avoid namespace pollution)
