//! Chat domain effects.
//!
//! Effects are thin orchestration layers that dispatch commands to handler functions.
//! All business logic lives in handler functions, not in the Effect trait implementation.

pub mod chat;
pub mod messaging;

pub use chat::ChatEffect;
pub use messaging::GenerateChatReplyEffect;
