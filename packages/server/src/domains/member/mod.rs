//! Member domain - handles member registration and management
//!
//! Architecture:
//!   API → HTTP handler → activities

pub mod activities;
pub mod data;
pub mod models;

// Re-export commonly used types
pub use data::MemberData;
pub use models::member::Member;
