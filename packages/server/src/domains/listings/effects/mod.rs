// Effects (side effects) for listings domain
//
// Effects are thin orchestrators that delegate to domain functions.
// Domain logic lives in separate function modules.

pub mod ai;
pub mod composite;
pub mod crawler;
pub mod deps;
pub mod discovery; // Static search queries for finding community resources
pub mod extraction; // Two-pass extraction: summarize pages, then synthesize listings
pub mod listing;
pub mod listing_extraction; // Domain functions for AI extraction
pub mod listing_operations; // Domain functions for listing CRUD operations
pub mod listing_report;
pub mod scraper;
pub mod sync;
pub mod syncing; // Domain functions for listing synchronization
pub mod utils;

pub use ai::*;
pub use composite::*;
pub use crawler::*;
pub use deps::*;
pub use discovery::{run_discovery_searches, DiscoveryResult, DISCOVERY_QUERIES, DEFAULT_LOCATION};
pub use listing::*;
pub use scraper::*;
pub use sync::*;
pub use utils::*;

// Domain function modules are available via:
//   - `effects::listing_extraction::*` - AI extraction functions
//   - `effects::listing_operations::*` - Listing CRUD operations
//   - `effects::syncing::*` - Listing synchronization functions
// (not re-exported at top level to avoid namespace pollution)
