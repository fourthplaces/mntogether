//! Member domain - handles member registration and management
//!
//! Architecture (Restate workflows):
//!   GraphQL → workflow_client.invoke(Workflow) → workflow orchestrates activities

pub mod activities;
pub mod data;
pub mod models;
pub mod restate;

// Re-export commonly used types
pub use data::MemberData;
pub use models::member::Member;
pub use restate::*;
