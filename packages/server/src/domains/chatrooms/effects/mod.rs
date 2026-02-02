//! Chat domain effects.
//!
//! Effects watch FACT events and call handlers directly for cascading.
//! NO *Requested events - GraphQL calls actions, effects call handlers on facts.

pub mod chat;
pub mod handlers;

pub use chat::chat_effect;
pub use handlers::{get_container_agent_config, handle_generate_greeting, handle_generate_reply};
