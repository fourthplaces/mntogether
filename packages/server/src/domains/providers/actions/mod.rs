//! Provider domain actions - business logic functions
//!
//! Actions are async functions called directly from GraphQL mutations via `process()`.
//! They do the work and can emit events for cascading effects.

mod mutations;
mod queries;

pub use mutations::*;
pub use queries::*;
