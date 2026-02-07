//! Chatrooms domain actions
//!
//! Actions contain business logic and are called from:
//! - GraphQL mutations via `process()` (entry points)
//!
//! AI response actions have moved to the agents domain.

mod entry_points;

// Entry-point actions - called directly from GraphQL mutations
pub use entry_points::{create_container, create_message, send_message};
