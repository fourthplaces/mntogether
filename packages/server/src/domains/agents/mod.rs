//! Agents domain - AI agent responses for chat containers.
//!
//! Watches ChatEvent facts and generates AI replies/greetings.
//! Publishes streaming tokens to StreamHub for real-time SSE delivery.
//!
//! Dependency direction: agents → chatrooms (reads/creates messages)
//! Chatrooms has no knowledge of agents.

pub mod actions;
pub mod effects;
pub mod events;
pub mod models;

pub use effects::agent_effect;
pub use events::ChatStreamEvent;
