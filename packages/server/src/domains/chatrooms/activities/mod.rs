//! Chatrooms domain actions
//!
//! Actions contain business logic and are called from:
//! - Restate virtual objects (entry points)
//!
//! AI response actions have moved to the agents domain.

mod entry_points;

// Entry-point actions - called from Restate virtual objects
pub use entry_points::{create_container, create_message, send_message};
