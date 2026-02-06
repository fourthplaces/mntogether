//! Member domain - handles member registration and management
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Components:
//! - actions: Entry-point business logic called directly from GraphQL via process()
//! - effects: Event handlers that respond to fact events

pub mod actions;
pub mod data;
pub mod effects;
pub mod events;
pub mod models;

// Re-export commonly used types
pub use data::MemberData;
pub use events::MemberEvent;
pub use models::member::Member;
