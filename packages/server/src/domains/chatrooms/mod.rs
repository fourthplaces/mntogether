//! Chatrooms domain - containers and messages for AI chat, comments, discussions.
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Components:
//! - actions: Entry-point business logic called directly from GraphQL via process()
//! - effects: Event handlers that respond to fact events
//! - models: Database models (Container, Message)
//! - data: GraphQL data types

pub mod actions;
pub mod data;
pub mod effects;
pub mod events;
pub mod models;

// Re-export commonly used types
pub use data::*;
pub use effects::chat_effect;
pub use events::ChatEvent;
pub use models::*;
