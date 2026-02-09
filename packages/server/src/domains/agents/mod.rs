//! Agents domain — autonomous entities with member identities.
//!
//! Role: `assistant` — responds to users in chat (greeting, reply generation)
//!
//! Dependency direction: agents → chatrooms (reads/creates messages)
//! Chatrooms has no knowledge of agents.

pub mod activities;
pub mod events;
pub mod models;

pub use events::ChatStreamEvent;
