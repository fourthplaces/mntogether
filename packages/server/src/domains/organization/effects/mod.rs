// Effects (side effects) for organization domain
//
// Effects are thin orchestrators that delegate to domain functions.
// Domain logic lives in separate function modules.
//
// Note: Most effects moved to listings domain. Organization domain now only
// provides utility functions for scraping.

pub mod scraping; // Domain functions for web scraping

// Re-export common utilities from the utils module
pub use crate::domains::organization::utils::*;

// Domain function modules are available via:
//   - `effects::scraping::*` - Web scraping functions
// (not re-exported at top level to avoid namespace pollution)
