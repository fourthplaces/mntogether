//! Chat domain effects.
//!
//! Effects are thin orchestration layers that dispatch events to action functions.
//! All business logic lives in action functions, not in the Effect trait implementation.

pub mod chat;

pub use chat::ChatEffect;
