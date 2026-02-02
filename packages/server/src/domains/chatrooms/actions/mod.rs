//! Chatrooms domain actions
//!
//! Entry-point actions are called directly from GraphQL mutations via `process()`.
//! They do the work, emit fact events, and return ReadResult.
//!
//! NOTE: Cascade handlers live in `effects/handlers.rs`, not here.
//! Actions are pure entry points - they mutate/read state and return values.

mod entry_points;

// Entry-point actions - called directly from GraphQL mutations, return values
pub use entry_points::{create_container, create_message, send_message};
