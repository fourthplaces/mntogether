//! Chatrooms domain actions
//!
//! Actions contain business logic and are called from:
//! - GraphQL mutations via `process()` (entry points)
//! - Effects for cascade workflows (ai_responses)

mod ai_responses;
mod entry_points;

// Entry-point actions - called directly from GraphQL mutations
pub use entry_points::{create_container, create_message, send_message};

// AI response actions - called from effects for agent responses
pub use ai_responses::{generate_greeting, generate_reply, get_container_agent_config};
