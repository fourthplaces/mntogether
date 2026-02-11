//! Provider domain actions - business logic functions
//!
//! Actions are async functions called from Restate virtual objects.
//! They do the work and return results directly.

mod mutations;
mod queries;

pub use mutations::*;
pub use queries::*;
