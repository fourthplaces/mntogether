// Effects (side effects) for organization domain
//
// Effects are thin orchestrators that delegate to domain functions.
// Domain logic lives in separate function modules.
//
// Note: Most effects moved to listings domain. Organization domain now only
// provides utility functions for scraping and source management.

pub mod scraping; // Domain functions for web scraping
pub mod source_operations; // Domain functions for source CRUD operations

// Re-export common utilities from the utils module
pub use crate::domains::organization::utils::*;

// Domain function modules are available via:
//   - `effects::scraping::*` - Web scraping functions
//   - `effects::source_operations::*` - Source CRUD operations
// (not re-exported at top level to avoid namespace pollution)
