//! Agent actions — AI response generation.
//!
//! Called from effects for cascade workflows (agent replies to user messages).

mod responses;

pub use responses::{
    generate_greeting, generate_reply, generate_reply_streaming, get_container_agent_config,
};
