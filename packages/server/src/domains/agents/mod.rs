//! Agents domain — autonomous entities with member identities and roles.
//!
//! Roles:
//! - `assistant`: responds to users in chat (greeting, reply generation)
//! - `curator`: discovers websites, extracts posts, enriches, monitors
//!
//! Dependency direction: agents → chatrooms (reads/creates messages)
//! Chatrooms has no knowledge of agents.

pub mod activities;
pub mod events;
pub mod models;
pub mod restate;

pub use events::ChatStreamEvent;
