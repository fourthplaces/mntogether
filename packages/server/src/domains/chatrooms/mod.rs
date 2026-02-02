//! Chatrooms domain - containers and messages for AI chat, comments, discussions.
//!
//! Architecture (seesaw 0.3.0):
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...
//!
//! Components:
//! - events: Request events (user intent) and fact events (what happened)
//! - actions: Business logic functions
//! - effects: Thin dispatcher that routes request events to actions
//! - edges/internal: React to fact events, emit new request events
//! - edges/mutation: GraphQL mutations that emit request events
//! - edges/query: GraphQL queries (read-only)

pub mod actions;
pub mod data;
pub mod edges;
pub mod effects;
pub mod events;
pub mod models;

// Re-export commonly used types
pub use data::*;
pub use effects::ChatEffect;
pub use events::ChatEvent;
pub use models::*;
