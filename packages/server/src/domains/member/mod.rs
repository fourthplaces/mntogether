//! Member domain - handles member registration and management
//!
//! Architecture (Restate workflows - TODO: complete migration):
//!   GraphQL → workflow_client.invoke(Workflow) → workflow orchestrates activities
//!
//! Components:
//! - activities: Business logic operations (renamed from actions)
//! - workflows: Durable workflow orchestrations (TODO: implement)
//! - effects: Legacy event handlers (TODO: remove after migration)
//! - events: Legacy fact events (TODO: remove after migration)

pub mod activities;
pub mod data;
pub mod effects; // TODO: Remove after migration
pub mod events; // TODO: Remove after migration
pub mod models;
pub mod workflows;

// Re-export commonly used types
pub use data::MemberData;
pub use events::MemberEvent; // TODO: Remove after migration
pub use models::member::Member;
pub use workflows::*;
