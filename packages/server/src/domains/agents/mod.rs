//! Agents domain - AI agent responses for chat containers.
//!
//! Generates AI replies/greetings and publishes streaming tokens
//! to StreamHub for real-time SSE delivery.
//!
//! Dependency direction: agents → chatrooms (reads/creates messages)
//! Chatrooms has no knowledge of agents.

pub mod activities;
pub mod events;
pub mod models;

pub use events::ChatStreamEvent;
