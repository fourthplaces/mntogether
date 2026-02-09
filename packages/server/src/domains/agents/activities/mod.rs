//! Agent activities — AI response generation for assistant agents.

mod responses;

pub use responses::{
    generate_greeting, generate_reply, get_container_agent_config,
};
