// Effects (side effects) for organization domain
//
// Effects are thin orchestrators that delegate to domain functions.
// Domain logic lives in separate function modules.

pub mod ai;
pub mod composite;
pub mod deps;
pub mod need;
pub mod need_extraction; // Domain functions for AI extraction
pub mod need_operations; // Domain functions for need CRUD operations
pub mod scraper;
pub mod scraping; // Domain functions for web scraping
pub mod submit;
pub mod sync;
pub mod syncing; // Domain functions for need synchronization
pub mod utils;

pub use ai::*;
pub use composite::*;
pub use deps::*;
pub use need::*;
pub use scraper::*;
pub use submit::*;
pub use sync::*;
pub use utils::*;

// Domain function modules are available via:
//   - `effects::need_extraction::*` - AI extraction functions
//   - `effects::need_operations::*` - Need CRUD operations
//   - `effects::scraping::*` - Web scraping functions
//   - `effects::syncing::*` - Need synchronization functions
// (not re-exported at top level to avoid namespace pollution)
