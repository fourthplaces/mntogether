//! Chat domain effects.
//!
//! Effects watch FACT events and call actions directly for cascade workflows.

pub mod chat;

pub use chat::chat_effect;
